#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

use std::ptr::NonNull;

#[cfg(not(target_os = "windows"))]
use libc::c_int as xsocket_t;
#[cfg(target_os = "windows")]
use std::os::windows::raw::SOCKET as xsocket_t;

// cbindgen --config cbindgen.toml --crate xaio
mod driver;

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
pub struct xring_s {
    // TODO: for uring-like: keep track of unsubmited and commit them before exaustion
}

/// Creates a new ring.
///
/// # Arguments
///   - `pring` `*pring` receives the new ring address or `NULL` on error.
///   - `opt_driver` driver to **move** to the ring or `NULL` to use the default driver.
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `pring == NULL`
///   -  `-ENOMEM` when the system is out of memory
#[no_mangle]
pub unsafe extern "C" fn xnew(pring: *mut *mut xring_s, opt_driver: *mut driver::xdriver_s) -> i32 {
    -libc::ENOMEM
}

/// Submit batched submissions.
///
/// # Arguments
///   - `ring` the completion ring
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `ring == NULL`
///   -  `-EBUSY` when the completion queue is full, the caller should call `xsubmit_and_wait(..., timeout_ms=0)` instead
///   -  `<0` the error code returned by the underlying subsystem
pub unsafe extern "C" fn xsubmit(ring: *mut xring_s) -> i32 {
    -libc::ENOSYS
}

/// Submit batched submissions then wait for up to `timeout_ms` for events, the wait will stop as soon as a completion event is present.
///
/// # Arguments
///   - `ring` the completion ring,
///   - `events` an array to receive the completion events,
///   - `capacity` the capacity of `events`,
///   - `timeout_ms` the maximum amount of time to wait for events or `<0` for infinity,
///
/// # Returns
///   -  `>0` the number of completion events stored in `events`
///   -  `0` on timeout
///   -  `-EINVAL` when `ring == NULL`
///   -  `-EINVAL` when `events == NULL`
///   -  `-EINVAL` when `capacity <= 0`
///   -  `<0` the error code returned by the underlying subsystem
#[no_mangle]
pub unsafe extern "C" fn xsubmit_and_wait(
    ring: *mut xring_s,
    events: *mut xevent_s,
    capacity: i32,
    timeout_ms: i32,
) -> i32 {
    -libc::ENOSYS
}

/// Tries to cancel the submission associated to the given token.
/// The submission associated to the token will still be retreived by `xring_wait` even
/// when this function returns `0`.
///
/// # Arguments
///   - `ring` the completion ring,
///   - `token` a token associated to a submissions,
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `ring == NULL`
///   -  `-EBUSY` when the completion queue is full, the caller should call `xsubmit_and_wait(..., timeout_ms=0)` and try again
///   -  `-ENOENT` when the submission associated to the token were not found
///   -  `-EALREADY` when the associated submission has progressed far enough that cancelation is no longer possible
#[no_mangle]
pub unsafe extern "C" fn xcancel(ring: *mut xring_s, token: u64, all: bool) -> i32 {
    -libc::ENOSYS
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
