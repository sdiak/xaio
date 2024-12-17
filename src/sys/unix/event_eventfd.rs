use crate::{libc_read_all, libc_write_all, selector::rawpoll, RawSocketFd};
use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::Arc,
};

pub type EventBuffer = u64;

/// An event can be used as an event wait/notify mechanism by user-space applications, and by the kernel to notify user-space applications of events.
///
/// An event starts not-notified.
#[repr(C)]
#[derive(Debug)]
pub struct Event {
    handle: Arc<OwnedFd>,
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.handle.as_raw_fd() == other.handle.as_raw_fd()
    }
}
impl Eq for Event {}
impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.as_raw_fd().hash(state);
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl Event {
    pub fn new() -> Result<Self> {
        let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if fd >= 0 {
            Ok(Self {
                handle: crate::catch_enomem(|| Arc::new(unsafe { OwnedFd::from_raw_fd(fd) }))?,
            })
        } else {
            Err(Error::last_os_error())
        }
    }
    /// Notify a waiter (multiple notification may be coalesced into one)
    pub fn notify(&self) -> Result<()> {
        let unpark_msg = 1u64.to_ne_bytes();
        libc_write_all(self.handle.as_raw_fd(), &unpark_msg, true)
    }

    #[inline]
    pub(crate) unsafe fn get_native_handle(&self) -> libc::c_int {
        self.handle.as_raw_fd()
    }

    /// Waits for the event to be notified or for `timeout_ms` milliseconds
    pub fn wait(&self, timeout_ms: i32) -> Result<()> {
        let fd = self.handle.as_raw_fd();
        let mut buffer = 0u64.to_ne_bytes();

        let pollfd = &mut [rawpoll::PollFD::new(RawSocketFd::new(fd), rawpoll::POLLIN)];
        loop {
            match libc_read_all(fd, &mut buffer, false) {
                Ok(_) => {
                    return Ok(());
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
