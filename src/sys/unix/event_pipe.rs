use crate::{libc_pipe2, libc_read_all, libc_write_all, selector::rawpoll};

use super::libc_close_log_on_error;
use std::io::{Error, ErrorKind, Result};

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
    read_end: libc::c_int,
    write_end: libc::c_int,
}
impl Drop for Inner {
    fn drop(&mut self) {
        libc_close_log_on_error(self.read_end);
        libc_close_log_on_error(self.write_end);
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
        let (read_end, write_end) = libc_pipe2(true, true)?;
        Ok(Self {
            handle: Arc::new(Inner {
                read_end,
                write_end,
            }),
        })
    }
    /// Notify a waiter (multiple notification may be coalesced into one)
    pub fn notify(&self) -> Result<()> {
        let unpark_msg = 1u8.to_ne_bytes();
        libc_write_all((*self.handle).write_end, &unpark_msg, true)
    }

    #[inline]
    pub(crate) unsafe fn native_handle(&self) -> libc::c_int {
        (*self.handle).read_end
    }

    /// Waits for the event to be notified or for `timeout_ms` milliseconds
    pub fn wait(&self, timeout_ms: i32) -> Result<()> {
        let fd = self.native_handle();
        let mut buffer = 0u8.to_ne_bytes();
        let pollfd = &mut [rawpoll::PollFD {
            fd: fd,
            events: rawpoll::POLLIN,
            revents: 0 as _,
        }];
        loop {
            match libc_read_all(fd, &mut buffer, false) {
                Ok(len) => {
                    debug_assert!(len == std::mem::sizeof::<buffer>());
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
