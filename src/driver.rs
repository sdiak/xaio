use super::{Request, RequestList};
use bitflags::bitflags;
use std::{pin::Pin, time::Duration};

pub trait Driver {
    fn name(&self) -> &'static str;

    fn config(&self) -> &DriverConfig;

    fn wait(
        &mut self,
        timeout: Option<Duration>,
        ready_list: &mut RequestList,
    ) -> std::io::Result<i32>;

    fn submit(&mut self, sub: Pin<&mut Request>) -> std::io::Result<()>;
    fn cancel(&mut self, sub: Pin<&Request>) -> std::io::Result<()>;
}

bitflags! {
    pub struct DriverFlags: u32 {
        const ATTACH_HANDLE = 1u32 << 0;
        const CLOSE_ON_EXEC = 1u32 << 1;
    }
}

/// IO Driver parameters
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DriverConfig {
    /// submission queue depth
    pub submission_queue_depth: u32,
    /// completion queue depth
    pub completion_queue_depth: u32,
    /// kernel busy-polling loop timeout in milliseconds, or 0 to deactivate kernel polling
    pub kernel_poll_timeout_ms: u32,
    /// Flags
    pub flags: u32,
    /// A sharable driver handle when (flags & DriverFlags::ATTACH_HANDLE)
    pub attach_handle: usize,
    /// An hint on the maximal number of file descriptor
    pub max_number_of_fd_hint: u32,
    /// An hint on the maximum number of io threads (Kernel or Userspace) or 0 for defaults
    pub max_number_of_threads: u32,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            submission_queue_depth: 64,
            completion_queue_depth: 128,
            kernel_poll_timeout_ms: 1000,
            flags: DriverFlags::CLOSE_ON_EXEC.bits(),
            attach_handle: usize::MAX,
            max_number_of_fd_hint: 256,
            max_number_of_threads: 0,
        }
    }
}
