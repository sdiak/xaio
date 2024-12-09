use crate::{libc_pipe2, libc_read_all, libc_write_all};

use super::{libc_close_log_on_error, selector::rawpoll};
use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
    os::fd::FromRawFd,
};

#[derive(Debug)]
pub(crate) struct DriverWaker {
    read_end: libc::c_int,
    write_end: libc::c_int,
}
impl Drop for DriverWaker {
    fn drop(&mut self) {
        libc_close_log_on_error(self.read_end);
        libc_close_log_on_error(self.write_end);
    }
}

impl DriverWaker {
    pub(crate) fn new() -> Result<Self> {
        let (read_end, write_end) = libc_pipe2(true, true)?;
        Ok(Self {
            read_end,
            write_end,
        })
    }
    #[inline]
    pub fn wake(&self) -> Result<()> {
        let unpark_msg = 1u8.to_le_bytes();
        libc_write_all(self.write_end, &unpark_msg, true)
    }
    #[inline]
    pub(crate) fn read_end(&self) -> libc::c_int {
        self.read_end
    }
    #[inline]
    pub(crate) fn drain(&self) {
        let mut sync = 0u128.to_le_bytes();
        while libc_read_all(self.read_end, &mut sync, false).is_ok() {}
    }
}
