/// Tries to reuse the sharable backend handle in `xdriver_params_s::attach_handle`
pub const XDRIVER_ATTACH_HANDLE: u32 = 0x00000001u32;

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
    reserved_: i32,
    /// A sharable backend handle when (flags & XDRIVER_ATTACH_HANDLE)
    attach_handle: usize,
}

/// IO Driver
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xdriver_s {
    params: xdriver_params_s,
}

extern "C" {
    /// Creates a new driver.
    ///
    /// # Arguments
    ///   - `pdriver` `*pdriver` receives the new driver address or `NULL` on error.
    ///
    /// # Returns
    ///   -  `0` on success
    ///   -  `-EINVAL` when `pdriver == NULL`
    ///   -  `-ENOMEM` when the system is out of memory
    pub fn xdriver_new(pdriver: *mut *mut xdriver_s) -> i32;
}
