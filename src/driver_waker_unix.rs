use super::selector::rawpoll;
use log::warn;
use std::{
    fs::File,
    io::{ErrorKind, Result, Write},
    os::fd::FromRawFd,
};

#[derive(Debug)]
pub(crate) struct DriverWaker {
    write_end: libc::c_int,
}

fn write_all(fd: libc::c_int, buf: &[u8], block_on_eagain: bool) -> Result<usize> {
    let mut file = unsafe { File::from_raw_fd(fd) };
    let mut done = 0;
    let todo = buf.len();
    while done < todo {
        match file.write(&buf[done..]) {
            Ok(n) => {
                done += n;
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    if block_on_eagain {
                        let pollfd = &mut [rawpoll::PollFD {
                            fd: fd,
                            events: rawpoll::POLLIN,
                            revents: 0 as _,
                        }];
                        rawpoll::sys_poll(pollfd, 5000)?;
                    } else {
                        return Err(e);
                    }
                }
                ErrorKind::Interrupted => {}
                _ => {
                    return Err(e);
                }
            },
        }
    }
    Ok(done)
}

impl DriverWaker {
    pub(crate) fn new(write_end: libc::c_int) -> Self {
        Self { write_end }
    }
    pub(crate) fn wake(&self) -> Result<()> {
        let unpark_msg = 1u64.to_le_bytes();
        write_all(self.write_end, &unpark_msg, true)?;
        Ok(())
    }
}
