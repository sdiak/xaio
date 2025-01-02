use super::stack::Stack;

#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
pub mod asm;

pub type UCtxStartCb = unsafe extern "C" fn(*mut ());
#[repr(C)]
pub struct UCtx {
    stack_pointer: *mut (),
}

extern "C" fn __unreachable() {
    eprintln!("Unreachable coroutine protection");
    std::process::abort();
}

unsafe extern "C" {
    unsafe fn __xaio_uctx_asm_boot();
    unsafe fn __xaio_uctx_asm_swap(from: *mut *mut (), to: *mut ());
    unsafe fn __xaio_uctx_asm_get_sp() -> *mut ();
    unsafe fn __xaio_uctx_asm_prefetch(sp: *const ());
}

impl UCtx {
    pub unsafe fn current() -> Self {
        Self {
            stack_pointer: unsafe { __xaio_uctx_asm_get_sp() },
        }
    }
    pub fn new(start_cb: UCtxStartCb, start_arg: *mut ()) -> Option<Self> {
        if let Some(stack) = Stack::new() {
            unsafe {
                let mut sp: *mut usize = stack.base.offset(stack.size) as _;
                // Leave 128 bytes at the top of the stack
                sp = sp.offset(-16);
                // Unreachable return address
                sp = sp.offset(-1);
                sp.write(__unreachable as usize);
                // Start argument
                sp = sp.offset(-1);
                sp.write(start_arg as usize);
                // Start callback
                sp = sp.offset(-1);
                sp.write(start_cb as usize);
                // The trampoline is necessary because we can't set rdi and rsi using just
                // a stack ; we use ASM to pop task_start_arg to rdi and call task_start_cb
                sp = sp.offset(-1);
                sp.write(__xaio_uctx_asm_boot as usize);
                // rbp, rbx, r12, r13, r14 and r15 in mbrt_uctx_asm_sysv_x86_64.S
                sp = sp.offset(-6);
                // WARNING: stack MUST be aligned on 16 bytes
                Some(Self {
                    stack_pointer: sp as *mut (),
                })
            }
        } else {
            None
        }
    }

    pub fn swap_to(&mut self, other: &mut UCtx) {
        println!("swap to {:?} {:?}", self.stack_pointer, other.stack_pointer);
        unsafe { __xaio_uctx_asm_swap(&mut self.stack_pointer, other.stack_pointer) };
        println!("ICI");
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use super::*;

    struct start_arg {
        thiz: Option<UCtx>,
        scheduler: *mut UCtx,
    }
    extern "C" fn start_scheduler_f(arg: *mut ()) {
        println!("start_scheduler_f {:?}", arg);
    }
    extern "C" fn start_f(arg: *mut ()) {
        println!("start {:?}", arg);
        let arg = unsafe { &mut *(arg as *mut start_arg) };
        let thiz = arg.thiz.as_mut().unwrap();
        println!("swap scheduler={:?}", arg.scheduler);
        thiz.swap_to(unsafe { &mut *arg.scheduler });
        println!("swap-end");
        thiz.swap_to(unsafe { &mut *arg.scheduler });
    }
    #[test]
    fn test_coro0() {
        let mut scheduler = unsafe { UCtx::current() };
        let arg: *mut start_arg = unsafe {
            std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(
                std::mem::size_of::<start_arg>(),
                std::mem::align_of::<start_arg>(),
            ))
        } as *mut start_arg;
        let arg = unsafe {
            std::ptr::write(
                arg,
                start_arg {
                    thiz: None,
                    scheduler: &mut scheduler as _,
                },
            );
            &mut *arg
        };
        arg.thiz = Some(UCtx::new(start_f, arg as *mut start_arg as _).unwrap());
        scheduler.swap_to(arg.thiz.as_mut().unwrap());
        scheduler.swap_to(arg.thiz.as_mut().unwrap());
    }
}
