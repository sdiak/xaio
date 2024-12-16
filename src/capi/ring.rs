
pub struct xring_s {
    // TODO: for uring-like: keep track of unsubmited and commit them before exaustion
}

/// Tries to reuse the sharable driver handle in `xdriver_params_s::attach_handle
pub const XCONFIG_FLAG_ATTACH_SINGLE_ISSUER: u32 = 1u32 << 0;
/// Clone or share resources with the given handle
pub const XCONFIG_FLAG_ATTACH_HANDLE: u32 = 1u32 << 1;
/// Close the handle on exec
pub const XCONFIG_FLAG_CLOSE_ON_EXEC: u32 = 1u32 << 2;
/// Use the most efficient polling mecanism available (for example io_uring will use epoll for polling)
pub const XCONFIG_FLAG_FAST_POLL: u32 = 1u32 << 3;

/// IO Driver parameters
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xconfig_s {
    /// submission queue depth
    pub submission_queue_depth: u32,
    /// completion queue depth
    pub completion_queue_depth: u32,
    /// kernel busy-polling loop timeout in milliseconds, a value of 0 deactivate kernel polling
    pub kernel_poll_timeout_ms: u32,
    /// Flags
    pub flags: u32,
    /// A sharable driver handle when (flags & XDRIVER_FLAG_ATTACH_HANDLE)
    pub attach_handle: usize,
    /// An hint on the maximal number of file descriptor
    pub max_number_of_fd_hint: u32,
    /// An hint on the maximum number of io threads (Kernel or Userspace) or 0 for defaults
    pub max_number_of_threads: u32,
}


/// Creates a new ring.
///
/// # Arguments
///   - `pring` `*pring` receives the new ring address or `NULL` on error.
///   - `opt_config` ring configuration or `NULL` to use the default configuration.
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `pring == NULL`
///   -  `-ENOMEM` when the system is out of memory
#[no_mangle]
pub unsafe extern "C" fn xnew(pring: *mut *mut xring_s, opt_config: *mut xconfig_s) -> i32 {
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
    events: *mut super::xevent_s,
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
