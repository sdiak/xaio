use crate::{libc_close_log_on_error, libc_read_all, libc_write_all, selector::rawpoll};
use std::{
    io::{Error, ErrorKind, Result},
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
#[repr(C)]
#[derive(Debug, Clone)]
struct Inner {
    fd: libc::c_int,
}
impl Drop for Inner {
    fn drop(&mut self) {
        libc_close_log_on_error(self.fd);
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
                handle: Arc::new(Inner { fd }),
            })
        } else {
            Err(Error::last_os_error())
        }
    }
    /// Notify a waiter (multiple notification may be coalesced into one)
    pub fn notify(&self) -> Result<()> {
        let unpark_msg = 1u64.to_ne_bytes();
        libc_write_all((*self.handle).fd, &unpark_msg, true)
    }

    #[inline]
    pub(crate) unsafe fn native_handle(&self) -> libc::c_int {
        (*self.handle).fd
    }

    /// Waits for the event to be notified or for `timeout_ms` milliseconds
    pub fn wait(&self, timeout_ms: i32) -> Result<()> {
        let fd = (*self.handle).fd;
        let mut buffer = 0u64.to_ne_bytes();
        let pollfd = &mut [rawpoll::PollFD {
            fd: fd,
            events: rawpoll::POLLIN,
            revents: 0 as _,
        }];
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
