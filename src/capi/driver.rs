/// Tries to reuse the sharable driver handle in `xdriver_params_s::attach_handle`
pub const XDRIVER_ATTACH_HANDLE: u32 = 0x00000001u32;

use crate::capi::{xaio_s, xcp_s};

/// IO Driver parameters
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_params_s {
    /// submission queue depth
    submission_queue_depth: i32,
    /// completion queue depth
    completion_queue_depth: i32,
    /// kernel busy-polling loop timeout in milliseconds, a value of <= 0 deactivate kernel polling
    kernel_poll_timeout_ms: i32,
    /// Flags
    flags: u32,
    /// An hint on the maximal number of file descriptor
    max_number_of_fd_hint: i32,
    reserved_: [i32; 9],
    /// A sharable driver handle when (flags & XDRIVER_ATTACH_HANDLE)
    attach_handle: usize,
}

const DD: usize = std::mem::size_of::<xdriver_params_s>();

/// IO Driver
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_s {
    clazz: *const xdriver_class_s,
    params: xdriver_params_s,
}

extern "C" {
    /// Creates a new driver.
    ///
    /// # Arguments
    ///   - `pdriver` `*pdriver` receives the new driver address or `NULL` on error.
    ///   - `opt_clazz` Optional clazz or `NULL` for defaults.
    ///   - `opt_params` Optional parameters hints or `NULL` for defaults.
    ///
    /// # Returns
    ///   -  `0` on success
    ///   -  `-EINVAL` when `pdriver == NULL`
    ///   -  `-ENOMEM` when the system is out of memory
    pub fn xdriver_new(
        pdriver: *mut *mut xdriver_s,
        opt_clazz: *const xdriver_class_s,
        opt_params: *const xdriver_params_s,
    ) -> i32;
}

type xdriver_is_supported_m = unsafe extern "C" fn() -> bool;
type xdriver_default_params_m =
    unsafe extern "C" fn(sizeof_params: usize, params: *mut xdriver_params_s) -> ();
type xdriver_open_m = unsafe extern "C" fn(thiz: *mut (), port: *const xcp_s) -> i32;
type xdriver_close_m = unsafe extern "C" fn(thiz: *mut (), port: *const xcp_s) -> ();
/// Get the driver native handle
///
/// # Arguments
///   - `thiz` the driver instance
///   - `phandle` `*phandle` receives the native handle on success
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `thiz == NULL`
///   -  `-EINVAL` when `phandle == NULL`
///   -  `-EBADF` when this driver is not backed by a native handle
type xdriver_get_native_handle_m =
    unsafe extern "C" fn(thiz: *const (), phandle: *mut usize) -> i32;

/// Wait for at least one event or for the given timeout to expire.
///
/// # Arguments
///   - `thiz` the driver instance
///   - `timeout_ms` give up after `timeout_ms` milliseconds when `timeout_ms >= 0` (otherwize wait forever)
///   - `events_sink` destination storage for the completed events
///
/// # Returns
///  - `>= 0` the number of completed events
///  - `< 0` an error descriptor
type xdriver_wait_m =
    unsafe extern "C" fn(thiz: *mut (), timeout: i32, events_sink: *mut ()) -> i32;

/// Submits a new asynchronous operation
///
/// # Arguments
///   - `thiz` the driver instance
///   - `op` the **moved** asynchronous operation, the driver owns this memory until it release it in `xdriver_class_s::wait`
///
/// # Returns
///  - `0` on success,
///  - `-ENOSYS` when the operation op-code is not supported,
///  - `-EBUSY` when the completion queue is full and the user should call `xdriver_class_s::wait`,
///  - `<= 0` any other error reported by the underlying system io multiplexer.
type xdriver_submit_m = unsafe extern "C" fn(thiz: *mut (), op: *mut xaio_s) -> i32;

/// Attemps to cancel an asynchronous operation,
/// when this method succeed, the operation will not appear in `xdriver_class_s::wait` and the caller owns the memory.
/// when this method fail, the operation will eventually appears in `xdriver_class_s::wait` with or without a status of `-ECANCELED`.
///
/// # Arguments
///   - `thiz` the driver instance
///   - `op` the asynchronous operation
///
/// # Returns
///  - `0` on success,
///  - `-ENOENT` when the operation was not found,
///  - `-EALREADY` when the operation is in progress,
///  - `-EBUSY` when the completion queue is full and the user should call `xdriver_class_s::wait`,
type xdriver_cancel_m = unsafe extern "C" fn(thiz: *mut (), op: *const xaio_s) -> i32;

/// IO Driver class
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_class_s {
    name: *const libc::c_char,
    flags: u32,
    instance_size: u32,

    is_supported: xdriver_is_supported_m,
    opt_default_params: xdriver_default_params_m,

    open: xdriver_open_m,
    close: xdriver_close_m,
    get_native_handle: xdriver_get_native_handle_m,

    wait: xdriver_wait_m,

    submit: xdriver_submit_m,
    cancel: xdriver_cancel_m,
}
