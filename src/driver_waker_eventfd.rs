use crate::{libc_read_all, libc_write_all, selector::rawpoll};

use super::libc_close_log_on_error;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub(crate) struct DriverWaker {
    evfd: libc::c_int,
}
impl Drop for DriverWaker {
    fn drop(&mut self) {
        libc_close_log_on_error(self.evfd);
    }
}

impl DriverWaker {
    pub(crate) fn new() -> Result<Self> {
        let evfd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if evfd >= 0 {
            Ok(Self { evfd })
        } else {
            Err(Error::last_os_error())
        }
    }
    #[inline]
    pub fn wake(&self) -> Result<()> {
        let unpark_msg = 1u64.to_ne_bytes();
        libc_write_all(self.evfd, &unpark_msg, true)
    }
    #[inline]
    pub(crate) fn read_end(&self) -> libc::c_int {
        self.evfd
    }
    #[inline]
    pub(crate) fn drain(&self) {}

    pub(crate) fn wait(&self, timeout_ms: i32) {
        let mut unpark_msg = 0u64.to_ne_bytes();
        let pollfd = &mut [rawpoll::PollFD {
            fd: self.evfd,
            events: rawpoll::POLLIN,
            revents: 0 as _,
        }];
        loop {
            match libc_read_all(self.evfd, &mut unpark_msg, false) {
                Ok(_) => {
                    return;
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    if let Err(pe) = rawpoll::sys_poll(pollfd, timeout_ms) {
                        log::warn!(
                            "Unexepected error in `DriverWaker::wait(&self, timeout_ms={timeout_ms}): {pe}"
                        );
                        return;
                    }
                }
                Err(err) => {
                    log::warn!(
                        "Unexepected error in `DriverWaker::wait(&self, timeout_ms={timeout_ms}): {err}"
                    );
                    return;
                }
            }
        }
    }
}
