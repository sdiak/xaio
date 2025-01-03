use std::{alloc::Layout, marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

mod sys;

pub struct UContext(NonNull<InnerErazed>);

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

    #[inline(always)]
    pub fn init(&mut self) -> bool {
        unsafe { self.0.as_mut().init() }
    }
    #[inline(always)]
    pub fn swap(&mut self, other: &Self) {
        println!("Swap: begin");
        unsafe { self.0.as_mut().swap(other.0.as_ref()) };
        println!("Swap: end");
    }
}

type StartCb = unsafe extern "C" fn(thiz: *mut InnerErazed);

const FLAG_LOCAL: usize = 1usize << 0;
const FLAG_STARTED: usize = 1usize << 1;
const FLAG_DONE: usize = 1usize << 2;

struct InnerErazed {
    flags: usize,
    stack_pointer: *mut (),
    stack: sys::Stack,
    start: StartCb,
}
impl InnerErazed {
    fn make(flags: usize, start: StartCb, size_hint: usize) -> Self {
        Self {
            flags,
            stack_pointer: std::ptr::null_mut(),
            stack: sys::Stack::with_size(size_hint),
            start,
        }
    }

    fn get() -> Self {
        Self {
            flags: FLAG_LOCAL | FLAG_STARTED,
            stack_pointer: unsafe { __xaio_uctx_asm_get_sp() } as _,
            stack: sys::Stack::root_stack(),
            start: Self::root_ctx_start,
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
        let start_arg = self as *mut Self as *mut ();
        sys::asm::setup_coroutine_on_stack(
            &mut self.stack,
            unsafe { std::mem::transmute::<StartCb, unsafe extern "C" fn(*mut ())>(self.start) },
            start_arg,
        );
        self.stack_pointer = self.stack.base() as _;
        self.flags |= FLAG_STARTED;
        true
    }

    #[inline(always)]
    fn swap(&mut self, other: &Self) {
        debug_assert!((self.flags & FLAG_STARTED) & (other.flags & FLAG_STARTED) != 0);
        unsafe { __xaio_uctx_asm_swap(&mut self.stack_pointer, other.stack_pointer) };
    }

    #[inline(always)]
    fn is_local(&self) -> bool {
        (self.flags & FLAG_LOCAL) != 0
    }

    #[inline(always)]
    fn is_root_ctx(&self) -> bool {
        self.stack.total_size() == 0
    }

    unsafe extern "C" fn root_ctx_start(_: *mut InnerErazed) {
        die("This should be unreachable");
    }
}

struct InnerLocal<F: FnOnce() + 'static> {
    as_inner: InnerErazed,
    f: MaybeUninit<F>,
}
impl<F: FnOnce() + 'static> InnerLocal<F> {
    const LAYOUT: Layout = unsafe {
        Layout::from_size_align_unchecked(std::mem::size_of::<Self>(), std::mem::align_of::<Self>())
    };

    fn make_with_size(f: F, stack_size_hint: usize) -> Option<NonNull<InnerErazed>> {
        let thiz = unsafe { std::alloc::alloc(Self::LAYOUT) } as *mut Self;
        if thiz.is_null() {
            None
        } else {
            unsafe {
                thiz.write(Self {
                    as_inner: InnerErazed::make(FLAG_LOCAL, Self::start, stack_size_hint),
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
    unsafe extern "C" fn start(thiz: *mut InnerErazed) {
        let thiz = unsafe { std::mem::transmute::<*mut InnerErazed, *mut Self>(thiz) };
        (unsafe { (&mut *thiz).f.assume_init_read() })();
    }
}

struct InnerShared<F: FnOnce() + Send + 'static> {
    as_inner: InnerErazed,
    f: MaybeUninit<F>,
}
impl<F: FnOnce() + Send + 'static> InnerShared<F> {
    const LAYOUT: Layout = unsafe {
        Layout::from_size_align_unchecked(std::mem::size_of::<Self>(), std::mem::align_of::<Self>())
    };
    fn make_with_size(f: F, stack_size_hint: usize) -> Option<NonNull<InnerErazed>> {
        let thiz = unsafe { std::alloc::alloc(Self::LAYOUT) } as *mut Self;
        if thiz.is_null() {
            None
        } else {
            unsafe {
                thiz.write(Self {
                    as_inner: InnerErazed::make(0, Self::start, stack_size_hint),
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
    unsafe extern "C" fn start(thiz: *mut InnerErazed) {
        let thiz = unsafe { std::mem::transmute::<*mut InnerErazed, *mut Self>(thiz) };
        (unsafe { (&mut *thiz).f.assume_init_read() })();
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

        root.swap(&uctx);
    }
}
