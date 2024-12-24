#![allow(dead_code)]
use crate::sys::{poll, PollEvent, PollFd};
use log;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::os::fd::FromRawFd;
use std::sync::{Mutex, MutexGuard};

pub(crate) fn read_all(fd: libc::c_int, buf: &mut [u8], block_on_eagain: bool) -> Result<()> {
    let mut file = std::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(fd) });
    let mut done = 0;
    let todo = buf.len();
    while done < todo {
        match file.read(&mut buf[done..]) {
            Ok(n) => {
                done += n;
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    if block_on_eagain {
                        let pollfd = &mut [PollFd::new(fd, PollEvent::IN)];
                        poll(pollfd, 5000)?;
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
    Ok(())
}

pub(crate) fn read(fd: libc::c_int, buf: &mut [u8], block_on_eagain: bool) -> Result<usize> {
    let mut file = std::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(fd) });
    let mut done = 0;
    let todo = buf.len();
    while done < todo {
        match file.read(&mut buf[done..]) {
            Ok(n) => {
                done += n;
                if n == 0 && done != 0 {
                    return Ok(done);
                }
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    if block_on_eagain && done == 0 {
                        let pollfd = &mut [PollFd::new(fd, PollEvent::IN)];
                        poll(pollfd, 5000)?;
                    } else if done != 0 {
                        return Ok(done);
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

pub(crate) fn write_all(fd: libc::c_int, buf: &[u8], block_on_eagain: bool) -> Result<()> {
    let mut file = std::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(fd) });
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
                        let pollfd = &mut [PollFd::new(fd, PollEvent::OUT)];
                        poll(pollfd, 5000)?;
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
    Ok(())
}

pub(crate) fn configure(fd: libc::c_int, non_blocking: bool, close_on_exec: bool) -> Result<()> {
    let mut tags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if tags < 0 {
        return Err(Error::last_os_error());
    }
    tags = if non_blocking {
        tags | libc::O_NONBLOCK
    } else {
        tags & !libc::O_NONBLOCK
    };
    if unsafe { libc::fcntl(fd, libc::F_SETFL, tags) < 0 } {
        return Err(Error::last_os_error());
    }
    tags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if tags < 0 {
        return Err(Error::last_os_error());
    }
    tags = if close_on_exec {
        tags | libc::FD_CLOEXEC
    } else {
        tags & !libc::FD_CLOEXEC
    };
    if unsafe { libc::fcntl(fd, libc::F_SETFD, tags) < 0 } {
        return Err(Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn dup(fd: libc::c_int) -> Result<libc::c_int> {
    let fd = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC) };
    if fd >= 0 {
        Ok(fd)
    } else {
        Err(Error::last_os_error())
    }
}

pub(crate) fn close_log_on_error(fd: libc::c_int) {
    if fd >= 0 && unsafe { libc::close(fd) } < 0 {
        log::warn!(
            "libc::close({}) failed: {:?}",
            fd,
            std::io::Error::last_os_error()
        );
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn libc_pipe2(
    non_blocking: bool,
    close_on_exec: bool,
) -> Result<(libc::c_int, libc::c_int)> {
    let mut flags = 0 as libc::c_int;
    if non_blocking {
        flags |= libc::O_NONBLOCK;
    }
    if close_on_exec {
        flags |= libc::O_CLOEXEC;
    }
    let mut fds: [libc::c_int; 2] = [-1, -1];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), flags) } >= 0 {
        Ok((fds[0], fds[1]))
    } else {
        Err(Error::last_os_error())
    }
}

#[cfg(not(target_os = "linux"))]

pub(crate) fn libc_pipe2(
    non_blocking: bool,
    close_on_exec: bool,
) -> Result<(libc::c_int, libc::c_int)> {
    let mut fds: [libc::c_int; 2] = [-1, -1];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } >= 0 {
        if let Err(e) = libc_configure_fd(fds[0], non_blocking, close_on_exec) {
            libc_close_log_on_error(fds[0]);
            libc_close_log_on_error(fds[1]);
            return Err(e);
        }
        if let Err(e) = libc_configure_fd(fds[1], non_blocking, close_on_exec) {
            libc_close_log_on_error(fds[0]);
            libc_close_log_on_error(fds[1]);
            return Err(e);
        }
        Ok((fds[0], fds[1]))
    } else {
        Err(Error::last_os_error())
    }
}

static CWD_LOCK: Mutex<()> = Mutex::<()>::new(());
struct AtHelper<'a> {
    guard: MutexGuard<'a, ()>,
    old_working_dir: std::path::PathBuf,
}

impl<'a> AtHelper<'a> {
    fn new(dirfd: libc::c_int) -> Result<AtHelper<'a>> {
        let thiz = AtHelper {
            guard: CWD_LOCK.lock().expect("Unrecoverable error"),
            old_working_dir: std::env::current_dir()?,
        };
        if unsafe { libc::fchdir(dirfd) } >= 0 {
            Ok(thiz)
        } else {
            Err(Error::last_os_error())
        }
    }
}
impl<'a> Drop for AtHelper<'a> {
    fn drop(&mut self) {
        std::env::set_current_dir(self.old_working_dir.clone()).expect("Unrecoverable error");
    }
}
pub(crate) fn with_dir<F: FnOnce() -> R, R>(dirfd: libc::c_int, f: F) -> R {
    let _guard = AtHelper::new(dirfd);
    f()
}
