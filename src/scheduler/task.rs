use std::{
    alloc::Layout,
    future::Future,
    io::Result,
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

pub(super) struct Task {
    future: BoxedFuture, // TODO: drop can only release Done futures
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

#[repr(C)]
pub struct xfuture_s {
    size: u32,
    align: u32,
    poll: xfuture_poll_cb,
}

#[repr(transparent)]
pub struct BoxedFuture {
    inner: NonNull<xfuture_s>,
}
impl Drop for BoxedFuture {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_mut() };
        // TODO: drop callback
        unsafe {
            std::alloc::dealloc(
                self.inner.as_ptr() as _,
                Layout::from_size_align_unchecked(inner.size as _, inner.align as _),
            )
        };
    }
}

struct BoxedShared<F: Future + Send + 'static> {
    as_future: xfuture_s,
    result: Option<F::Output>,
    f: F,
}
impl<F: Future + Send + 'static> BoxedShared<F> {
    unsafe extern "C" fn trampoline(thiz: &mut xfuture_s, cx: &mut Context<'static>) -> i32 {
        let thiz = &mut *(thiz as *mut xfuture_s as *mut libc::c_void as *mut BoxedShared<F>);
        let f = Pin::new_unchecked(&mut thiz.f);
        if let Poll::Ready(result) = f.poll(cx) {
            thiz.result = Some(result);
            0
        } else {
            i32::MIN
        }
    }
    pub fn boxed(f: F) -> Option<BoxedFuture> {
        let layout = std::alloc::Layout::new::<BoxedShared<F>>();
        let ptr = unsafe { std::alloc::alloc(layout) } as *mut BoxedShared<F>;
        if !ptr.is_null() {
            let mem = unsafe { &mut *ptr };
            mem.as_future.size = layout.size() as _;
            mem.as_future.align = layout.align() as _;
            mem.as_future.poll = BoxedShared::<F>::trampoline;
            mem.result = None;
            unsafe {
                std::ptr::write::<F>(&mut mem.f as *mut F, f);
            };
            Some(BoxedFuture {
                inner: unsafe { NonNull::new_unchecked(ptr as *mut libc::c_void as _) },
            })
        } else {
            None
        }
    }
}
