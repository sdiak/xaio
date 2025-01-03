use std::{alloc::Layout, marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

mod sys;

#[repr(transparent)]
pub struct UContext(NonNull<InnerErazed>);
impl Drop for UContext {
    fn drop(&mut self) {
        let ptr = self.0.as_ptr();
        unsafe {
            std::ptr::drop_in_place(ptr);
            std::alloc::dealloc(ptr as _, (&*ptr).vtable.layout);
        }
    }
}

impl UContext {
    pub fn pinned<F: FnOnce() + 'static>(f: F, stack_size_hint: usize) -> Option<Self> {
        InnerLocal::make_with_size(f, stack_size_hint).map(Self)
    }
    pub fn movable<F: FnOnce() + Send + 'static>(f: F, stack_size_hint: usize) -> Option<Self> {
        InnerShared::make_with_size(f, stack_size_hint).map(Self)
    }

    pub fn default_size() -> usize {
        sys::Stack::DEFAULT_TOTAL_SIZE - sys::Stack::guard_size()
    }

    pub fn get() -> Option<Self> {
        let inner = unsafe {
            std::alloc::alloc(Layout::from_size_align_unchecked(
                std::mem::size_of::<InnerErazed>(),
                std::mem::align_of::<InnerErazed>(),
            ))
        } as *mut InnerErazed;
        if inner.is_null() {
            None
        } else {
            unsafe { inner.write(InnerErazed::get()) };
            Some(Self(unsafe { NonNull::new_unchecked(inner) }))
        }
    }

    pub fn set_exit_context(&mut self, ctx: Option<&Self>) {
        unsafe {
            self.0.as_mut().exit_context = ctx.map(|cx| cx.0);
        };
    }

    #[inline(always)]
    pub fn init(&mut self) -> bool {
        unsafe { self.0.as_mut().init() }
    }
    #[inline(always)]
    pub fn swap(&mut self, other: &mut Self) {
        // println!("Swap: begin {:?}=>{:?}", self.0.as_ptr(), other.0.as_ptr());
        unsafe { self.0.as_mut().swap(other.0.as_mut()) };
        // println!("Swap: end");
    }
    #[inline(always)]
    pub fn is_movable(&self) -> bool {
        !unsafe { self.0.as_ref().is_local() }
    }
}

type StartCb = unsafe extern "C" fn(thiz: *mut InnerErazed);
type DropErasedCb = unsafe extern "C" fn(thiz: *mut InnerErazed);

const FLAG_LOCAL: usize = 1usize << 0;
const FLAG_STARTED: usize = 1usize << 1;
const FLAG_DONE: usize = 1usize << 2;
thread_local! {
    static CURRENT_CTX: std::cell::Cell<*const InnerErazed>  = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

struct VTable {
    start: StartCb,
    drop_erased: DropErasedCb,
    layout: Layout,
}
struct InnerErazed {
    vtable: &'static VTable,
    flags: usize,
    stack_pointer: *mut (),
    exit_context: Option<NonNull<InnerErazed>>,
    stack: sys::Stack,
}
impl Drop for InnerErazed {
    fn drop(&mut self) {
        unsafe { (self.vtable.drop_erased)(self as _) };
    }
}
impl InnerErazed {
    const ROOT_VTABLE: VTable = VTable {
        start: Self::root_ctx_start,
        drop_erased: Self::root_ctx_drop_erased,
        layout: unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<Self>(),
                std::mem::align_of::<Self>(),
            )
        },
    };
    fn make(vtable: &'static VTable, flags: usize, size_hint: usize) -> Self {
        Self {
            vtable,
            flags,
            stack_pointer: std::ptr::null_mut(),
            exit_context: None,
            stack: sys::Stack::with_size(size_hint),
        }
    }

    const fn get() -> Self {
        Self {
            vtable: &Self::ROOT_VTABLE,
            flags: FLAG_LOCAL | FLAG_STARTED,
            stack_pointer: 0xDEADBEEF as usize as _,
            exit_context: None,
            stack: sys::Stack::root_stack(),
        }
    }

    fn init(&mut self) -> bool {
        assert!((self.flags & FLAG_STARTED) == 0);
        if !self
            .stack
            .allocate(&mut sys::StackPool::new(sys::Stack::DEFAULT_TOTAL_SIZE))
        {
            return false;
        }
        self.stack.register();
        let start_arg = self as *mut Self as *mut ();
        self.stack_pointer = sys::asm::setup_coroutine_on_stack(
            &mut self.stack,
            unsafe {
                std::mem::transmute::<StartCb, unsafe extern "C" fn(*mut ())>(self.vtable.start)
            },
            start_arg,
        );
        self.flags |= FLAG_STARTED;
        true
    }

    #[inline(always)]
    fn swap(&mut self, other: &mut Self) {
        debug_assert!((self.flags & FLAG_STARTED) & (other.flags & FLAG_STARTED) != 0);

        // println!(
        //     "-Swap: begin {:?}=>{:?}",
        //     self.stack_pointer, other.stack_pointer
        // );
        // unsafe { other.caller = Some(NonNull::new_unchecked(self as _)) };
        CURRENT_CTX.set(other as _);
        unsafe { __xaio_uctx_asm_swap(&mut self.stack_pointer, other.stack_pointer) };
    }
    fn start_epilog(&mut self) {
        self.flags |= FLAG_DONE;
        if let Some(exit_context) = self.exit_context.as_mut() {
            let caller = unsafe { exit_context.as_mut() };
            self.swap(caller);
        } else {
            die("Coroutine exited without a defined exit-context");
        }
    }

    #[inline(always)]
    fn is_local(&self) -> bool {
        (self.flags & FLAG_LOCAL) != 0
    }

    #[inline(always)]
    fn is_root_ctx(&self) -> bool {
        self.stack.total_size() == 0
    }

    unsafe extern "C" fn root_ctx_drop_erased(_: *mut InnerErazed) {}
    unsafe extern "C" fn root_ctx_start(_: *mut InnerErazed) {
        die("This should be unreachable");
    }
}

struct InnerLocal<F: FnOnce() + 'static> {
    as_inner: InnerErazed,
    f: MaybeUninit<F>,
}
impl<F: FnOnce() + 'static> InnerLocal<F> {
    const VTABLE: VTable = VTable {
        start: Self::start,
        drop_erased: Self::drop_erased,
        layout: unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<Self>(),
                std::mem::align_of::<Self>(),
            )
        },
    };

    fn make_with_size(f: F, stack_size_hint: usize) -> Option<NonNull<InnerErazed>> {
        let thiz = unsafe { std::alloc::alloc(Self::VTABLE.layout) } as *mut Self;
        if thiz.is_null() {
            None
        } else {
            unsafe {
                thiz.write(Self {
                    as_inner: InnerErazed::make(&Self::VTABLE, FLAG_LOCAL, stack_size_hint),
                    f: MaybeUninit::new(f),
                })
            };
            Some(unsafe {
                NonNull::new_unchecked(std::mem::transmute::<*mut Self, *mut InnerErazed>(thiz))
            })
        }
    }
    fn make(f: F) -> Option<NonNull<InnerErazed>> {
        Self::make_with_size(f, sys::Stack::DEFAULT_TOTAL_SIZE - sys::Stack::guard_size())
    }
    unsafe extern "C" fn drop_erased(thiz: *mut InnerErazed) {
        let thiz = unsafe { &mut *std::mem::transmute::<*mut InnerErazed, *mut Self>(thiz) };
        println!("drop_erased1");
        if (thiz.as_inner.flags & FLAG_DONE) == 0 {
            println!("drop_erased2");
            thiz.f.assume_init_drop();
        }
    }
    unsafe extern "C" fn start(thiz: *mut InnerErazed) {
        let thiz = unsafe { &mut *std::mem::transmute::<*mut InnerErazed, *mut Self>(thiz) };
        (unsafe { thiz.f.assume_init_read() })();
        thiz.as_inner.start_epilog();
    }
}

struct InnerShared<F: FnOnce() + Send + 'static> {
    as_inner: InnerErazed,
    f: MaybeUninit<F>,
}
impl<F: FnOnce() + Send + 'static> InnerShared<F> {
    const VTABLE: VTable = VTable {
        start: Self::start,
        drop_erased: Self::drop_erased,
        layout: unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<Self>(),
                std::mem::align_of::<Self>(),
            )
        },
    };
    fn make_with_size(f: F, stack_size_hint: usize) -> Option<NonNull<InnerErazed>> {
        let thiz = unsafe { std::alloc::alloc(Self::VTABLE.layout) } as *mut Self;
        if thiz.is_null() {
            None
        } else {
            unsafe {
                thiz.write(Self {
                    as_inner: InnerErazed::make(&Self::VTABLE, 0, stack_size_hint),
                    f: MaybeUninit::new(f),
                })
            };
            Some(unsafe {
                NonNull::new_unchecked(std::mem::transmute::<*mut Self, *mut InnerErazed>(thiz))
            })
        }
    }
    fn make(f: F) -> Option<NonNull<InnerErazed>> {
        Self::make_with_size(f, sys::Stack::DEFAULT_TOTAL_SIZE - sys::Stack::guard_size())
    }
    unsafe extern "C" fn drop_erased(thiz: *mut InnerErazed) {
        let thiz = unsafe { &mut *std::mem::transmute::<*mut InnerErazed, *mut Self>(thiz) };
        println!("drop_erased1");
        if (thiz.as_inner.flags & FLAG_DONE) == 0 {
            println!("drop_erased2");
            thiz.f.assume_init_drop();
        }
    }
    unsafe extern "C" fn start(thiz: *mut InnerErazed) {
        let thiz = unsafe { &mut *std::mem::transmute::<*mut InnerErazed, *mut Self>(thiz) };
        (unsafe { thiz.f.assume_init_read() })();
        thiz.as_inner.start_epilog();
    }
}

pub(crate) fn die(message: &str) -> ! {
    log::error!("{}, aborting.", message);
    eprintln!("{}, aborting.", message);
    std::process::abort();
}

unsafe extern "C" {
    unsafe fn __xaio_uctx_asm_swap(from: *mut *mut (), to: *mut ());
    unsafe fn __xaio_uctx_asm_get_sp() -> *mut ();
    unsafe fn __xaio_uctx_asm_prefetch(sp: *const ());
}

#[inline]
#[cold]
fn cold() {}

#[inline]
fn likely(b: bool) -> bool {
    if !b {
        cold()
    }
    b
}

#[inline]
fn unlikely(b: bool) -> bool {
    if b {
        cold()
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coro() {
        let mut root = UContext::get().unwrap();
        let mut uctx = UContext::pinned(|| println!("Run"), UContext::default_size()).unwrap();
        assert!(uctx.init());
        uctx.set_exit_context(Some(&root));
        root.swap(&mut uctx);

        let mut uctx = UContext::movable(|| println!("Run"), UContext::default_size()).unwrap();
        assert!(uctx.init());
        uctx.set_exit_context(Some(&root));
        root.swap(&mut uctx);
    }
}
