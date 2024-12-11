use super::selector::rawpoll;
use std::{
    fs::File,
    io::{Error, ErrorKind, Read, Result, Write},
};

#[cfg(target_family = "unix")]
use std::os::fd::FromRawFd;
#[cfg(target_family = "windows")]
use std::os::windows::io::FromRawHandle;

#[allow(dead_code)]
pub(crate) fn libc_close_log_on_error(fd: libc::c_int) {
    if fd >= 0 && unsafe { libc::close(fd) } < 0 {
        log::warn!(
            "libc::close({}) failed: {:?}",
            fd,
            std::io::Error::last_os_error()
        );
    }
}

#[cfg(target_family = "windows")]
#[allow(dead_code)]
pub(crate) fn windows_close_handle_log_on_error(handle: windows_sys::Win32::Foundation::HANDLE) {
    if handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
        && unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) } == 0
    {
        log::warn!(
            "windows::CloseHandle({:?}) failed: {:?}",
            handle,
            std::io::Error::last_os_error()
        );
    }
}

#[cfg(target_family = "unix")]
#[allow(dead_code)]
pub(crate) fn libc_read_all(fd: libc::c_int, buf: &mut [u8], block_on_eagain: bool) -> Result<()> {
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
                        let pollfd = &mut [rawpoll::PollFD {
                            fd,
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
    Ok(())
}

#[cfg(target_family = "unix")]
#[allow(dead_code)]
pub(crate) fn libc_read(fd: libc::c_int, buf: &mut [u8], block_on_eagain: bool) -> Result<usize> {
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
                        let pollfd = &mut [rawpoll::PollFD {
                            fd,
                            events: rawpoll::POLLIN,
                            revents: 0 as _,
                        }];
                        rawpoll::sys_poll(pollfd, 5000)?;
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

#[cfg(target_family = "unix")]
#[allow(dead_code)]
pub(crate) fn libc_write_all(fd: libc::c_int, buf: &[u8], block_on_eagain: bool) -> Result<()> {
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
                        let pollfd = &mut [rawpoll::PollFD {
                            fd,
                            events: rawpoll::POLLOUT,
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
    Ok(())
}

#[cfg(target_family = "unix")]
#[allow(dead_code)]
pub(crate) fn libc_configure_fd(
    fd: libc::c_int,
    non_blocking: bool,
    close_on_exec: bool,
) -> Result<()> {
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

#[cfg(target_os = "linux")]
#[allow(dead_code)]
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

#[cfg(all(not(target_os = "linux"), target_family = "unix"))]
#[allow(dead_code)]
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
