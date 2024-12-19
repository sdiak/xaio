use std::usize;

pub struct xring_s {
    // TODO: for uring-like: keep track of unsubmited and commit them before exaustion
    group_inner: *const libc::c_void,
    index_in_group: usize,
    join_handle: std::thread::JoinHandle<()>,
}

pub struct xring_group_s {

}

// #[repr(C)]
// pub struct xreq_s {
//     #[cfg(target_family = "windows")]
//     _win_header: windows_sys::Win32::System::IO::OVERLAPPED,
//     #[cfg(target_family = "unix")]
//     _unix_header: usize, // ??1 : pinned to this ring, ??0 : pinned to this scheduler
    
//     pub(crate) status: std::sync::atomic::AtomicI32,
//     pub(crate) flags_and_op_code: u32,
// };

#[repr(C, packed)]
pub struct OpNoOp {}
#[repr(C, packed)]
pub struct OpDeadlineF {
    // f1: u8, f2: u16, f3: u32, deadline: u64,
    deadline: u64,
}
#[repr(u8)]
pub enum TestLayout {
    OpNoOp(OpNoOp),
    OpDeadline(OpDeadlineF),
}
pub struct xreq_t {
    #[cfg(target_family = "windows")]
    _win_header: windows_sys::Win32::System::IO::OVERLAPPED,
    #[cfg(target_family = "unix")]
    _unix_header: usize,
    list_next: std::sync::atomic::AtomicUsize,
    status: std::sync::atomic::AtomicI32,
    flags: u16,
    _reverved8: u8,
    op: TestLayout
}
const _: () = assert!(std::mem::size_of::<xreq_t>() == 32);

#[no_mangle]
pub unsafe extern "C" fn get_f1(_tl: &TestLayout) -> u8 {
    0
    // match tl {
    //     TestLayout::OpNoOp(op) => op.f1,
    //     TestLayout::OpDeadline(op) => op.f1,
    //     // TestLayout::OpDeadline(&v) 
    //     // OpNoOp(&op) => op.f1,
    // }
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
impl Default for xconfig_s {
    fn default() -> Self {
        Self {
            submission_queue_depth: 64,
            completion_queue_depth: 256,
            kernel_poll_timeout_ms: 0,
            flags: XCONFIG_FLAG_FAST_POLL | XCONFIG_FLAG_CLOSE_ON_EXEC,
            attach_handle: usize::MAX,
            max_number_of_fd_hint: 1024,
            max_number_of_threads: 0,
        }
    }
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

/// Submit a prepared request.
///
/// # Arguments
///   - `ring` the completion ring
///   - `req` the **moved** request ; the ring is the only owner until the request is returned by `xwait`.
///   - `flush` flush batched request
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `ring == NULL`
///   -  `-EINVAL` when `req == NULL` or when the request is invalid
///   -  `-EBUSY` when the completion queue is full, the caller should call `xsubmit_and_wait(..., timeout_ms=0)` instead
///   -  `<0` the error code returned by the underlying subsystem
pub unsafe extern "C" fn xsubmit_prepared(
    ring: *mut xring_s,
    req: *mut crate::Request,
    flush: bool,
) -> i32 {
    -libc::ENOSYS
}

/// Submit a prepared request batch.
///
/// # Arguments
///   - `ring` the completion ring
///   - `batch` the **moved** requests ; the ring is the only owner of those requests until they are returned by `xwait`. In case of success,
/// batch will be empty after the return of this function ; in case of error, the batch is left unchanged.
///   - `linked` when `true` the ring will execute the request one after the other. If one operation fails, subsequent
/// operation will fail with a status of `-ECANCELED`
///   - `flush` flush batched request
///
/// # Returns
///   -  `0` on success
///   -  `-EINVAL` when `ring == NULL`
///   -  `-EINVAL` when `batch == NULL`
///   -  `-EINVAL` when one of the request is invalid
///   -  `-EBUSY` when the completion queue is full, the caller should call `xsubmit_and_wait(..., timeout_ms=0)` instead
///   -  `<0` the error code returned by the underlying subsystem
pub unsafe extern "C" fn xsubmit_prepared_batch(
    ring: *mut xring_s,
    batch: *mut super::xreq_fifo_s,
    linked: bool,
    flush: bool,
) -> i32 {
    -libc::ENOSYS
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
pub unsafe extern "C" fn xwait(
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
