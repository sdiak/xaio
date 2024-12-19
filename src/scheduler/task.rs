use std::{
    alloc::Layout,
    future::Future,
    i32,
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{AtomicI32, Ordering},
    task::{Context, Poll},
};

pub(super) struct Task {
    future: BoxedFuture,
    /// The receiver `Executor` address (ored with `1usize` when pinned to it)
    completion_queue: usize,
}
// `Task` can be sent across threads safely because it ensures that
// the underlying `Future` type isn't touched unless it's `Send`.
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

// pub struct xwaker_s {
//     addr: *const (),
//     vtable: *const (),
// }
/// Work callback.
///
/// # Arguments
///   - `work_arg` argument passed to `xring_submit_work`
///
/// # Returns
///   -  `i32::MIN` when the future is still pending
pub type xfuture_poll_cb =
    unsafe extern "C" fn(thiz: &mut xfuture_s, cx: &mut Context<'static>) -> i32;
/// Release resources associated to the given future (**DO NOT** deallocate `thiz`)
pub type xfuture_drop_cb = unsafe extern "C" fn(thiz: &mut xfuture_s);

#[repr(C)]
pub struct xfuture_s {
    flags: u32,
    size: u32,
    align: u32,
    status: AtomicI32,
    poll: xfuture_poll_cb,
    drop: xfuture_drop_cb,
}

#[repr(transparent)]
pub struct BoxedFuture {
    inner: NonNull<xfuture_s>,
}
impl Drop for BoxedFuture {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_mut() };
        if inner.status.swap(-libc::ECANCELED, Ordering::Acquire) != i32::MIN {
            // We can drop the inner memory
            unsafe {
                (inner.drop)(inner);
                std::alloc::dealloc(
                    self.inner.as_ptr() as _,
                    Layout::from_size_align_unchecked(inner.size as _, inner.align as _),
                )
            };
        } // Otherwize the loop will do the drop TODO:
    }
}

struct BoxedShared<F: Future + Send + 'static> {
    as_future: xfuture_s,
    result: Option<F::Output>,
    f: F,
}
impl<F: Future + Send + 'static> BoxedShared<F> {
    unsafe extern "C" fn poll_trampoline(thiz: &mut xfuture_s, cx: &mut Context<'static>) -> i32 {
        let thiz = &mut *(thiz as *mut xfuture_s as *mut libc::c_void as *mut BoxedShared<F>);
        let f = Pin::new_unchecked(&mut thiz.f);
        if let Poll::Ready(result) = f.poll(cx) {
            thiz.result = Some(result);
            0
        } else {
            i32::MIN
        }
    }
    unsafe extern "C" fn drop_trampoline(thiz: &mut xfuture_s) {
        let _ = *(thiz as *mut xfuture_s as *mut libc::c_void as *mut BoxedShared<F>);
    }
    pub fn boxed(f: F) -> Option<BoxedFuture> {
        let layout = std::alloc::Layout::new::<BoxedShared<F>>();
        assert!(layout.size() <= u32::MAX as _ && layout.align() <= u32::MAX as _); // TODO: const
        let ptr = unsafe { std::alloc::alloc(layout) } as *mut BoxedShared<F>;
        if !ptr.is_null() {
            let mem = unsafe { &mut *ptr };
            mem.as_future.size = layout.size() as _;
            mem.as_future.align = layout.align() as _;
            mem.as_future.poll = BoxedShared::<F>::poll_trampoline;
            mem.result = None;
            unsafe {
                std::ptr::write::<F>(&mut mem.f as *mut F, f);
                std::ptr::write::<AtomicI32>(
                    &mut mem.as_future.status as *mut AtomicI32,
                    AtomicI32::new(i32::MIN),
                );
            };
            Some(BoxedFuture {
                inner: unsafe { NonNull::new_unchecked(ptr as *mut libc::c_void as _) },
            })
        } else {
            None
        }
    }
}
