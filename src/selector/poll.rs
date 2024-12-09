use std::collections::HashMap;
use std::os::raw::c_short;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::Event;
use super::Interest;
use super::SelectorImpl;
use crate::RawSocketFd;

#[cfg_attr(not(target_os = "windows"), path = "rawpoll_unix.rs")]
#[cfg_attr(target_os = "windows", path = "rawpoll_windows.rs")]
mod rawpoll;

pub use rawpoll::{POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};

pub struct Poll {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    poll_fds: Vec<rawpoll::PollFD>,
    tokens: Vec<usize>,
    fd_to_index: HashMap<RawSocketFd, u32>,
    len: u32,
}

fn interests_to_events(interests: Interest) -> libc::c_short {
    let mut events = 0 as libc::c_short;
    if (interests & Interest::READABLE).bits() != 0u32 {
        events |= rawpoll::POLLIN;
    }
    if (interests & Interest::WRITABLE).bits() != 0u32 {
        events |= rawpoll::POLLOUT;
    }
    if (interests & Interest::PRIORITY).bits() != 0u32 {
        events |= rawpoll::POLLPRI;
    }
    events
}
/*
impl Inner {
    fn try_reserve(&mut self, additional: usize) -> std::io::Result<()> {
        if self.poll_fds.try_reserve(additional).is_err()
            || self.tokens.try_reserve(additional).is_err()
            || self.fd_to_index.try_reserve(additional).is_err()
        {
            return Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory));
        }
        Ok(())
    }
    fn register(
        &mut self,
        fd: RawSocketFd,
        token: usize,
        interests: Interest,
    ) -> std::io::Result<()> {
        if !fd.is_valid() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        if self.fd_to_index.contains_key(&fd) {
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }
        if self.len == u32::MAX {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
        }
        let mut index = self.len as usize;
        if index < self.poll_fds.len() {
            index = 0;
            for pfd in self.poll_fds.iter() {
                if !RawSocketFd::new(pfd.fd as _).is_valid() {
                    break;
                }
                index += 1;
            }
            self.poll_fds[index].fd = fd.inner as _;
            self.poll_fds[index].events = interests_to_events(interests);
        } else {
            self.try_reserve(1)?;
            self.poll_fds.push(rawpoll::PollFD {
                fd: fd.inner as _,
                events: interests_to_events(interests),
                revents: 0 as libc::c_short,
            });
        }
        self.tokens[index] = token;
        self.fd_to_index.insert(fd, index as u32);
        self.len += 1;
        Ok(())
    }

    fn reregister(
        &mut self,
        fd: RawSocketFd,
        token: usize,
        interests: Interest,
    ) -> std::io::Result<()> {
        if !fd.is_valid() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        if let Some(index) = self.fd_to_index.get(&fd) {
            let index = *index as usize;
            self.poll_fds[index].events = interests_to_events(interests);
            self.tokens[index] = token;
            self.len -= 1;
            return Ok(());
        }
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    fn unregister(&mut self, fd: RawSocketFd) -> std::io::Result<()> {
        if fd.is_valid() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        if let Some(index) = self.fd_to_index.remove(&fd) {
            let index = index as usize;
            self.poll_fds[index].fd = RawSocketFd::invalid().inner as _;
            self.len -= 1;
            return Ok(());
        }
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    fn select(&mut self, events: &mut [Event], timeout: libc::c_int) -> std::io::Result<usize> {
        if events.len() > 0 {
            rawpoll::sys_poll(&mut self.poll_fds, timeout)
        } else {
            Ok(0usize)
        }
    }
}

impl SelectorImpl for Poll {
    fn register(
        &mut self,
        fd: RawSocketFd,
        token: usize,
        interests: Interest,
    ) -> std::io::Result<()> {
        self.inner
            .lock()
            .expect("Lock misuse")
            .register(fd, token, interests)
    }

    fn reregister(
        &mut self,
        fd: RawSocketFd,
        token: usize,
        interests: Interest,
    ) -> std::io::Result<()> {
        self.inner
            .lock()
            .expect("Lock misuse")
            .reregister(fd, token, interests)
    }

    fn unregister(&mut self, fd: RawSocketFd) -> std::io::Result<()> {
        self.inner.lock().expect("Lock misuse").unregister(fd)
    }

    fn select(&self, events: &mut [Event], timeout: Option<Duration>) -> std::io::Result<usize> {
        let timeout = timeout
            .map(|to| {
                // `Duration::as_millis` truncates, so round up. This avoids
                // turning sub-millisecond timeouts into a zero timeout, unless
                // the caller explicitly requests that by specifying a zero
                // timeout.
                to.checked_add(Duration::from_nanos(999_999))
                    .unwrap_or(to)
                    .as_millis() as libc::c_int
            })
            .unwrap_or(-1 as libc::c_int);
        self.inner
            .lock()
            .expect("Lock misuse")
            .select(events, timeout)
    }
}
 */