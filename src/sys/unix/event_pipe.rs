use crate::{
    libc_close_log_on_error, libc_pipe2, libc_read, libc_write_all, selector::rawpoll, RawSocketFd,
};

use std::{
    io::{Error, ErrorKind, Result},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::Arc,
};

/// An event can be used as an event wait/notify mechanism by user-space applications, and by the kernel to notify user-space applications of events.
///
/// An event starts not-notified.
#[repr(C)]
#[derive(Debug)]
pub struct Event {
    handle: Arc<Inner>,
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.handle.read_end.as_raw_fd() == other.handle.read_end.as_raw_fd()
    }
}
impl Eq for Event {}
impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.read_end.as_raw_fd().hash(state);
    }
}
#[repr(C)]
#[derive(Debug)]
struct Inner {
    read_end: OwnedFd,
    write_end: OwnedFd,
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
        let (read_end, write_end) = libc_pipe2(true, true)?;
        Ok(Self {
            handle: crate::catch_enomem(|| {
                Arc::new(Inner {
                    read_end: unsafe { OwnedFd::from_raw_fd(read_end) },
                    write_end: unsafe { OwnedFd::from_raw_fd(write_end) },
                })
            })?,
        })
    }
    /// Notify a waiter (multiple notification may be coalesced into one)
    pub fn notify(&self) -> Result<()> {
        let unpark_msg = 1u8.to_ne_bytes();
        libc_write_all((*self.handle).write_end.as_raw_fd(), &unpark_msg, true)
    }

    #[inline]
    pub(crate) unsafe fn get_native_handle(&self) -> libc::c_int {
        (*self.handle).read_end.as_raw_fd()
    }

    /// Waits for the event to be notified or for `timeout_ms` milliseconds
    pub fn wait(&self, timeout_ms: i32) -> Result<()> {
        let fd = (*self.handle).read_end.as_raw_fd();
        let mut buffer = [0u8; 256];
        let pollfd = &mut [rawpoll::PollFD::new(RawSocketFd::new(fd), rawpoll::POLLIN)];
        loop {
            match libc_read(fd, &mut buffer, false) {
                Ok(len) => {
                    debug_assert!(len > 0);
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
