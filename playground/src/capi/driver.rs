/// Tries to reuse the sharable driver handle in `xdriver_params_s::attach_handle
pub const XDRIVER_FLAG_ATTACH_HANDLE: u32 = 0x00000001u32;
pub const XDRIVER_FLAG_CLOSE_ON_EXEC: u32 = 0x00000002u32;

use std::{alloc::Layout, default, ffi::c_void, pin::Pin, ptr::NonNull};

use crate::capi::{xaio_s, xcp_s};

/// IO Driver parameters
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_params_s {
    /// submission queue depth
    pub submission_queue_depth: i32,
    /// completion queue depth
    pub completion_queue_depth: i32,
    /// kernel busy-polling loop timeout in milliseconds, a value of <= 0 deactivate kernel polling
    pub kernel_poll_timeout_ms: i32,
    /// Flags
    pub flags: u32,
    /// A sharable driver handle when (flags & XDRIVER_FLAG_ATTACH_HANDLE)
    pub attach_handle: usize,
    /// An hint on the maximal number of file descriptor
    pub max_number_of_fd_hint: i32,
    pub reserved_: i32,
}
#[no_mangle]
pub unsafe extern "C" fn xdriver_params_default(params: NonNull<xdriver_params_s>) {
    params.write(xdriver_params_s::default());
}

impl Default for xdriver_params_s {
    fn default() -> Self {
        Self {
            submission_queue_depth: 64,
            completion_queue_depth: 128,
            kernel_poll_timeout_ms: 1000,
            flags: XDRIVER_FLAG_CLOSE_ON_EXEC,
            attach_handle: usize::MAX,
            max_number_of_fd_hint: 256,
            reserved_: 0,
        }
    }
}

/// IO Driver
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_s {
    pub(super) clazz: &'static xdriver_class_s,
    pub(super) params: xdriver_params_s,
}

#[no_mangle]
pub unsafe extern "C" fn xdriver_class_default() -> &'static xdriver_class_s {
    todo!()
}

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
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn xdriver_new(
    pdriver: *mut *mut xdriver_s,
    opt_clazz: Option<&'static xdriver_class_s>,
    opt_params: Option<&xdriver_params_s>,
) -> i32 {
    if pdriver.is_null() {
        return -libc::EINVAL;
    }
    *pdriver = std::ptr::null_mut();
    let clazz = opt_clazz.unwrap_or(xdriver_class_default());

    let mut params_memory = xdriver_params_s::default();
    let params = opt_params.unwrap_or(&*(clazz.default_params)(&mut params_memory));

    let driver: *mut xdriver_s = std::alloc::alloc_zeroed(
        Layout::from_size_align(clazz.instance_size, clazz.instance_align as _)
            .expect("Invalid layout"),
    ) as _;
    if driver.is_null() {
        return -libc::ENOMEM;
    }
    (*driver).clazz = clazz;
    (*driver).params = *params;
    *pdriver = driver;
    0
}

pub(crate) extern "C" fn xdriver_is_supported() -> bool {
    true
}
pub(crate) extern "C" fn driver_is_not_supported() -> bool {
    true
}

type xdriver_is_supported_m = unsafe extern "C" fn() -> bool;
type xdriver_default_params_m =
    unsafe extern "C" fn(params: &mut xdriver_params_s) -> &mut xdriver_params_s;
type xdriver_open_m = unsafe extern "C" fn(thiz: &mut c_void, port: &xcp_s) -> i32;
type xdriver_close_m = unsafe extern "C" fn(thiz: &mut c_void, port: &xcp_s) -> ();
/// Get the driver native handle
///
/// # Arguments
///   - `thiz` the driver instance
///   - `phandle` `*phandle` receives the native handle on success
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `thiz == NULL`
///   -  `-EBADF` when this driver is not backed by a native handle
type xdriver_get_native_handle_m = unsafe extern "C" fn(thiz: &c_void, phandle: &mut usize) -> i32;

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
    unsafe extern "C" fn(thiz: &mut c_void, timeout: i32, events_sink: &mut c_void) -> i32;

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
type xdriver_submit_m = unsafe extern "C" fn(thiz: &mut c_void, op: Pin<&mut xaio_s>) -> i32;

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
type xdriver_cancel_m = unsafe extern "C" fn(thiz: &mut c_void, op: Pin<&xaio_s>) -> i32;

/// IO Driver class
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_class_s {
    pub(crate) name: *const libc::c_char,
    pub(crate) flags: u32,
    pub(crate) instance_align: u32,
    pub(crate) instance_size: usize,

    pub(crate) is_supported: xdriver_is_supported_m,
    pub(crate) default_params: xdriver_default_params_m,

    pub(crate) open: xdriver_open_m,
    pub(crate) close: xdriver_close_m,
    pub(crate) get_native_handle: xdriver_get_native_handle_m,

    pub(crate) wait: xdriver_wait_m,

    pub(crate) submit: xdriver_submit_m,
    pub(crate) cancel: xdriver_cancel_m,
}
