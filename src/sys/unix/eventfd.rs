use crate::{libc_read_all, libc_write_all, selector::rawpoll, RawSocketFd};
use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

/// An event can be used as an event wait/notify mechanism by user-space applications, and by the kernel to notify user-space applications of events.
///
/// An event starts not-notified.
#[repr(C)]
#[derive(Debug)]
pub struct RawEventFd {
    handle: libc::c_int,
}
impl AsRawFd for RawEventFd {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.handle
    }
}
impl Drop for RawEventFd {
    fn drop(&mut self) {
        if self.handle > -1 {
            let _ = unsafe { OwnedFd::from_raw_fd(self.handle) };
        }
    }
}
impl PartialEq for RawEventFd {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}
impl Eq for RawEventFd {}
impl std::hash::Hash for RawEventFd {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl RawEventFd {
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
            handle: super::dup(self.handle)?,
        })
    }

    pub fn write(&self, value: u64) -> Result<()> {
        let buf = value.to_ne_bytes();
        libc_write_all(self.handle, &buf, true)
    }

    pub fn read(&self, timeout_ms: i32) -> Result<u64> {
        let fd = self.handle;
        let mut buffer = 0u64.to_ne_bytes();

        let pollfd = &mut [rawpoll::PollFD::new(RawSocketFd::new(fd), rawpoll::POLLIN)];
        loop {
            match libc_read_all(fd, &mut buffer, false) {
                Ok(_) => {
                    return Ok(u64::from_ne_bytes(buffer));
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    rawpoll::sys_poll(pollfd, timeout_ms)?;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }
}
