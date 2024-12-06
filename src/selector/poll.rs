use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::SelectorImpl;
use crate::RawSocketFd;
use super::Interest;
use super::Event;

#[cfg_attr(not(target_os = "windows"), path = "rawpoll_unix.rs")]
#[cfg_attr(target_os = "windows", path = "rawpoll_windows.rs")]
mod rawpoll;

pub struct Poll {
    inner: Arc<Mutex<Inner>>
}

struct Inner {
    poll_fds: Vec<rawpoll::PollFD>,
    tokens: Vec<usize>,
    fd_to_index: HashMap<RawSocketFd, u32>,
    len: u32
    //TODO: event fd, ..., for notify on another ??
}

fn interests_to_events(interests: Interest) -> i16 {
    let mut events = rawpoll::POLLERR | rawpoll::POLLHUP;
    if interests.is_readable() {
        events |= rawpoll::POLLIN;
    }
    if interests.is_writable() {
        events |= rawpoll::POLLOUT;
    }
    if interests.is_priority() {
        events |= rawpoll::POLLPRI;
    }
    events
}
impl Inner {
    fn try_reserve(&mut self, additional: usize) -> std::io::Result<()> {
        if self.poll_fds.try_reserve(additional).is_err() || self.tokens.try_reserve(additional).is_err() || self.fd_to_index.try_reserve(additional).is_err() {
            return Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory));
        }
        Ok(())
    }
    fn register(&mut self, fd: RawSocketFd, token: usize, interests: Interest) -> std::io::Result<()> {
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
            self.poll_fds[index].fd = fd.inner as usize;
            self.poll_fds[index].events = interests_to_events(interests);
            self.poll_fds[index].revents = 0i16;
        } else {
            self.try_reserve(1)?;
            self.poll_fds.push(rawpoll::PollFD { fd: fd.inner as _, events: interests_to_events(interests), revents:0i16 });
        }
        self.tokens[index] = token;
        self.fd_to_index.insert(fd, index as u32);
        self.len += 1;
        Ok(())
    }
    fn reregister(&mut self, fd: RawSocketFd, token: usize, interests: Interest) -> std::io::Result<()> {
        if !fd.is_valid() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        if let Some(index) = self.fd_to_index.get(&fd) {
            let index = *index as usize;
            self.poll_fds[index].events = interests_to_events(interests);
            self.poll_fds[index].revents = 0i16;
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
            self.poll_fds[index].events = 0i16;
            self.poll_fds[index].revents = 0i16;
            self.len -= 1;
            return Ok(());
        }
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

impl SelectorImpl for Poll {
    fn register(&mut self, fd: RawSocketFd, token: usize, interests: Interest) -> std::io::Result<()> {
        self.inner.lock().expect("Lock misuse").register(fd, token, interests)
    }
    fn reregister(&mut self, fd: RawSocketFd, token: usize, interests: Interest) -> std::io::Result<()> {
        self.inner.lock().expect("Lock misuse").reregister(fd, token, interests)
    }
    fn unregister(&mut self, fd: RawSocketFd) -> std::io::Result<()> {
        self.inner.lock().expect("Lock misuse").unregister(fd)
    }
    fn select(&self, events: &mut [Event], timeout: Option<Duration>) -> std::io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(-1)) // TODO:
    }
}