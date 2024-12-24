use super::ioutils::{close_log_on_error, dup, read_all, write_all};
use crate::sys::{self, PollEvent, PollFd};

use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    os::fd::AsRawFd,
};

/// An event can be used as an event wait/notify mechanism by user-space applications, and by the kernel to notify user-space applications of events.
///fR
/// An event starts not-notified.
#[repr(C)]
#[derive(Debug)]
pub struct EventFd {
    pub(crate) handle: sys::RawFd,
}
impl AsRawFd for EventFd {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.handle
    }
}
impl Drop for EventFd {
    fn drop(&mut self) {
        if self.handle > -1 {
            close_log_on_error(self.handle);
        }
    }
}
impl PartialEq for EventFd {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}
impl Eq for EventFd {}
impl std::hash::Hash for EventFd {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl EventFd {
    pub fn invalid() -> Self {
        Self { handle: -1 }
    }
    pub fn new(initval: libc::c_uint, semaphore: bool) -> Result<Self> {
        let mut flags = libc::EFD_CLOEXEC | libc::EFD_NONBLOCK;
        if semaphore {
            flags |= libc::EFD_SEMAPHORE;
        }
        let handle = unsafe { libc::eventfd(initval, flags) };
        if handle >= 0 {
            Ok(Self { handle })
        } else {
            Err(Error::last_os_error())
        }
    }
    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            handle: dup(self.handle)?,
        })
    }

    pub fn write(&self, value: u64) -> Result<()> {
        let buf = value.to_ne_bytes();
        write_all(self.handle, &buf, true)
    }

    pub fn read(&self, timeout_ms: i32) -> Result<u64> {
        let fd = self.handle;
        let mut buffer = 0u64.to_ne_bytes();

        let pollfd = &mut [PollFd::new(fd, PollEvent::IN)];
        loop {
            match read_all(fd, &mut buffer, false) {
                Ok(_) => {
                    return Ok(u64::from_ne_bytes(buffer));
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    sys::poll(pollfd, timeout_ms)?;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }
}
