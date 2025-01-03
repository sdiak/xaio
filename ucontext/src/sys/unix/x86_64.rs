// use super::UCtxRunCb;

pub(crate) type StartCb = unsafe extern "C" fn(*mut ());

extern "C" fn __unreachable() -> ! {
    crate::die("Unreachable coroutine protection");
}
unsafe extern "C" {
    unsafe fn __xaio_uctx_asm_boot();
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        std::arch::global_asm!(include_str!("asm/x86_64_sysv_macho.S")); // TODO:
    } else {
        std::arch::global_asm!(include_str!("asm/x86_64_sysv_elf.S"));
        pub(crate) fn setup_coroutine_on_stack(stack: &mut crate::sys::Stack, start_cb: StartCb, start_arg: *mut ()) {
            unsafe {
                let mut sp: *mut usize = stack.base().offset(stack.size() as isize) as _;
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
            }
        }
    }
}

// pub(super) fn uctx_new(start_cb: UCtxRunCb, start_arg: *mut libc::c_void) -> UCtx {

// }
