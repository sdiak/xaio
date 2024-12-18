#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

use std::{mem::ManuallyDrop, ptr::NonNull};

#[cfg(not(target_os = "windows"))]
use libc::c_int as xsocket_t;
#[cfg(target_os = "windows")]
use std::os::windows::raw::SOCKET as xsocket_t;

// cbindgen --config cbindgen.toml --crate xaio
mod driver;

pub mod ring;
pub use ring::*;

use crate::stat;

#[cfg_attr(target_os = "linux", path = "driver_epoll_linux.rs")]
#[cfg_attr(not(target_os = "linux"), path = "driver_epoll_unsupported.rs")]
mod rawpoll;

/// A thread-local completion port
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xcp_s {
    prv__thread_id: usize,
    // prv__driver:
    prv__id: u32,
    prv__refcount: u32,
    prv__now: i64,
}

#[repr(C)]
#[derive(Debug)]
pub struct xaio_s {
    prv__cp: *mut xcp_s,
    prv__status: i32,
    prv__flags_and_op_code: u32,
    prv__next: std::sync::atomic::AtomicUsize,
}

extern "C" {
    /// Creates a new completion port bound to the current thread.
    ///
    /// # Arguments
    ///   - `pport` `*pport` receives a new completion port address or `NULL` on error.
    ///   - `opt_driver` driver to **move** to the port or `NULL` to use the default driver.
    ///
    /// # Returns
    ///   -  `0` on success
    ///   -  `-EINVAL` when `pport == NULL`
    ///   -  `-ENOMEM` when the system is out of memory
    #[must_use]
    pub fn xcp_new(pport: *mut *mut xcp_s) -> i32;
}

#[repr(C)]
pub struct xevent_s {
    pub status: i32,
    pub flags: u32,
    pub token: u64,
}

#[repr(C)]
pub struct xreq_fifo_s {
    front: Option<NonNull<crate::Request>>,
    back: Option<NonNull<crate::Request>>,
}

/// Work callback.
///
/// # Arguments
///   - `work_arg` argument passed to `xring_submit_work`
///
/// # Returns
///   -  `>=0` on success
///   -  `<0` on error
pub type xwork_cb = extern "C" fn(work_arg: *mut libc::c_void) -> i32;

/// Submit some work to the IO thread pool
///
/// # Arguments
///   - `ring` the completion ring,
///   - `token` a token associated to the submission,
///   - `work_cb` the work function pointer,
///   - `work_arg` the argument to pass to `work_cb`,
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `ring == NULL`
///   -  `-EBUSY` when the submission queue or the completion queue is full, the caller
/// should call `xsubmit_and_wait` and try again
///   -  `-EEXIST` when `token` is already associated to a submission
///   -  `-EINVAL` when `work_cb == (xwork_cb)0`
///   -  `-ENOMEM` when the system is out of memory
///   -  `<0` the error code returned by the underlying subsystem
#[no_mangle]
pub unsafe extern "C" fn xio_work(
    ring: *mut xring_s,
    token: u64,
    work_cb: xwork_cb,
    work_arg: *mut libc::c_void,
) -> i32 {
    -libc::ENOSYS
}

#[no_mangle]
pub unsafe extern "C" fn xsend(ring: *mut xring_s, token: u64, socket: xsocket_t) -> i32 {
    -libc::ENOSYS
}


pub const WAKER_SIZE: usize = std::mem::size_of::<std::task::Waker>();

#[repr(C)]
pub struct xtask_waker_s {
    data: *const libc::c_void,
    vtable: *const libc::c_void,
}
#[repr(C)]
pub struct xcontext_s {
    waker: xtask_waker_s
}

pub type xfuture_poll_cb = unsafe extern "C" fn(thiz: &mut xfuture_s) -> i32;
#[repr(C)]
pub struct xfuture_s {
    poll: xfuture_poll_cb,
    cx: xcontext_s,
}

#[no_mangle]
pub unsafe extern "C" fn xfuture_s(f: &mut xfuture_s) -> i32 {
    (f.poll)(f)
}

impl std::future::Future for xfuture_s {
    type Output = i32;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let thiz = unsafe { self.get_unchecked_mut() };
        unsafe { std::mem::transmute::<&mut xtask_waker_s, &mut std::task::Waker>(&mut thiz.cx.waker).clone_from(cx.waker()) };
        let status = unsafe { (thiz.poll)(thiz) };
        if status == i32::MIN {
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(status)
        }
    }
}