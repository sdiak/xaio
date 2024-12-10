#[cfg(target_os = "linux")]
use super::driver_epoll::DriverEPoll;
#[cfg(target_os = "windows")]
use super::driver_iocp_windows::DriverIOCP;
use super::driver_none::DriverNone;
#[cfg(target_os = "linux")]
use super::driver_uring::DriverURing;

use super::{Request, RequestList};
use bitflags::bitflags;
use enum_dispatch::enum_dispatch;
use std::{pin::Pin, time::Duration};

use std::io::Result;

#[cfg(not(target_family = "windows"))]
pub type DriverHandle = libc::c_int;
#[cfg(not(target_family = "windows"))]
pub const AN_INVALID_DRIVER_HANDLE: DriverHandle = -1 as _;
#[cfg(target_family = "windows")]
pub type DriverHandle = windows_sys::Win32::Foundation::HANDLE;
#[cfg(target_family = "windows")]
pub const AN_INVALID_DRIVER_HANDLE: DriverHandle =
    windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

#[enum_dispatch]
pub trait DriverIFace {
    fn name(&self) -> &'static str;

    fn config(&self) -> &DriverConfig;

    fn wait(&mut self, timeout_ms: i32, ready_list: &mut RequestList) -> Result<i32>;

    fn submit(&mut self, sub: Pin<&mut Request>) -> Result<()>;
    fn cancel(&mut self, sub: Pin<&Request>) -> Result<()>;

    fn wake(&self) -> Result<()>;

    fn get_native_handle(&self) -> DriverHandle;
}

#[allow(clippy::large_enum_variant)]
#[enum_dispatch(DriverIFace)]
pub enum Driver {
    // #[cfg(target_os = "linux")]
    // DriverURing,
    #[cfg(target_os = "linux")]
    DriverEPoll,
    // TODO: DriverKQueue
    #[cfg(target_os = "windows")]
    DriverIOCP,
    // TODO: DriverPoll
    DriverNone,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum DriverKind {
    URing,
    EPoll,
    KQueue,
    IOCP,
    Poll,
    None,
}
impl DriverKind {
    pub fn name(&self) -> &'static str {
        match self {
            DriverKind::URing => "URing",
            DriverKind::EPoll => "EPoll",
            DriverKind::KQueue => "KQueue",
            DriverKind::IOCP => "IOCP",
            DriverKind::Poll => "Poll",
            DriverKind::None => "None",
        }
    }
}

impl Driver {
    pub fn new(kind: DriverKind, config: &DriverConfig) -> Result<Box<Self>> {
        match kind {
            //DriverKind::URing => Ok(Box::new(Driver::from(DriverURing::new(config)?))),
            #[cfg(target_os = "linux")]
            DriverKind::EPoll => Ok(Box::new(Driver::from(DriverEPoll::new(config)?))),
            //DriverKind::KQueue => Ok(Box::new(Driver::from(DriverKQueue::new(config)?))),
            #[cfg(target_os = "windows")]
            DriverKind::IOCP => Ok(Box::new(Driver::from(DriverIOCP::new(config)?))),
            // DriverKind::Poll => Ok(Box::new(Driver::from(DriverPoll::new(config)?))),
            _ => Ok(Box::new(Driver::from(DriverNone::new(
                config,
                Some(kind.name()),
            )?))),
        }
    }
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
