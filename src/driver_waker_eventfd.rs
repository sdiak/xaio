use crate::libc_write_all;

use super::{libc_close_log_on_error, selector::rawpoll};
use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
    os::fd::FromRawFd,
};

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
        let unpark_msg = 1u64.to_le_bytes();
        libc_write_all(self.evfd, &unpark_msg, true)
    }
    #[inline]
    pub(crate) fn read_end(&self) -> libc::c_int {
        self.evfd
    }
    #[inline]
    pub(crate) fn drain(&self) {}
}
