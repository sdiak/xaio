use rustc_hash::{FxHashMap, FxHashSet};

use crate::selector::rawpoll::{sys_poll, PollFD, POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};
use crate::selector::{Event, Interest};
use crate::RawSocketFd;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::io::{Error, ErrorKind, Result};
use std::sync::MutexGuard;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

pub struct Poll {
    inner: Arc<Inner>,
}
struct Inner {
    waker: crate::sys::Event,
    /// sys_poll argument
    registrations: RefCell<Registration>, // Locked first
    pollfds: RefCell<PollFds>, // Locked second
}

struct PollFds {
    active_fds: usize,
    fds: Vec<PollFD>,
}
impl PollFds {
    fn active_fds(&self) -> usize {
        self.active_fds
    }
    fn len(&self) -> usize {
        self.fds.len()
    }
    fn add_fd(&mut self, fd: RawSocketFd, interests: i16) -> Result<usize> {
        let mut index = self.active_fds;
        if self.active_fds == self.fds.len() {
            self.fds.try_reserve(1)?;
            self.fds.push(PollFD::new(fd, interests));
        } else {
            index = 0;
            for pfd in self.fds.iter_mut() {
                if pfd.is_disabled() {
                    pfd.enable(fd, interests);
                    break;
                }
                index += 1;
            }
            assert!(index > 0 && index < self.active_fds);
        }
        self.active_fds += 1;
        Ok(index)
    }
    fn mod_fd(&mut self, index: usize, interests: i16) {
        self.fds[index].set_events(interests);
    }
    fn rem_fd(&mut self, index: usize) {
        let mut index = index;
        assert!(index > 0); // Can not remove waker
        self.active_fds -= 1;
        if index == self.active_fds {
            self.fds.remove(index);
            index -= 1;
            // Cleanup tail tombstones
            while index > 0 && self.fds[index].is_disabled() {
                self.fds.remove(index);
                index -= 1;
            }
        } else {
            self.fds[index].disable();
        }
    }
}

impl Poll {}

impl Inner {
    fn register(&self, fd: RawSocketFd, token: usize, interests: i16) -> Result<()> {
        if !fd.is_valid() {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        // Grab the registrations lock
        let mut registrations = self.registrations.borrow_mut();

        // Do not register twice
        if registrations.entries.contains_key(&fd) {
            return Err(Error::from(ErrorKind::AlreadyExists));
        }
        registrations
            .entries
            .try_reserve(1)
            .or(Err(Error::from(ErrorKind::OutOfMemory)))?;

        // Grab the pollfds lock
        let mut pollfds = self.pollfds.borrow_mut();
        let index = pollfds.add_fd(fd, interests)?;

        registrations.entries.insert(
            fd,
            FdEntry {
                index_in_fds: index,
                token,
            },
        );
        Ok(())
    }
    fn reregister(&self, fd: RawSocketFd, token: usize, interests: i16) -> Result<()> {
        if !fd.is_valid() {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        // Grab the registrations lock
        let mut registrations = self.registrations.borrow_mut();

        // Find the entry
        if let Some(entry) = registrations.entries.get_mut(&fd) {
            entry.token = token;
            // Grab the pollfds lock and update
            self.pollfds
                .borrow_mut()
                .mod_fd(entry.index_in_fds, interests);
            Ok(())
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    }
    fn unregister(&self, fd: RawSocketFd) -> std::io::Result<()> {
        // Grab the registrations lock
        let mut registrations = self.registrations.borrow_mut();

        // Find the entry
        if let Some(entry) = registrations.entries.remove(&fd) {
            // Grab the pollfds lock and remove
            self.pollfds.borrow_mut().rem_fd(entry.index_in_fds);
            Ok(())
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    }
    /// Returns `Ok(true)` when the wakup was not done by a registration wake event
    fn select_inner(
        &self,
        events: &mut Vec<crate::selector::Event>,
        timeout_ms: i32,
    ) -> Result<bool> {
        // Grab the registrations lock
        let registrations = self.registrations.borrow();
        // Grab the pollfds lock
        let mut pollfds = self.pollfds.borrow_mut();

        let mut n_events = sys_poll(&mut pollfds.fds, timeout_ms)?;
        for pfd in pollfds.fds.iter() {
            if n_events == 0 {
                break;
            }
            // TODO: waker !
            if pfd.revents() != 0 as _ {
                events.push(Event {
                    events: PollFD::events_to_interest(pfd.revents()),
                    token: registrations.entries.get(&pfd.fd()).unwrap().token as _,
                });
                n_events -= 1;
            }
        }
        Ok(true)
    }
    fn poll(&self, events: &mut Vec<crate::selector::Event>) -> Result<usize> {
        events.clear();
        while events.len() == 0 && !self.select_inner(events, 0)? {}
        Ok(events.len())
    }
    fn wait(&self, events: &mut Vec<crate::selector::Event>, timeout_ms: i32) -> Result<usize> {
        debug_assert!(timeout_ms != 0);
        let timeout_ms = if timeout_ms > 0 { timeout_ms } else { i32::MAX };
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as _);
        events.clear();
        while events.len() == 0 && timeout_ms > 0 {
            self.select_inner(events, timeout_ms)?;
            if events.len() == 0 && std::time::Instant::now() >= deadline {
                return Ok(0usize);
            }
        }
        Ok(events.len())
    }
}

#[derive(Debug, Clone)]
struct Registration {
    /// maps an fd to its index in fds and it's associated token
    entries: FxHashMap<RawSocketFd, FdEntry>,
    /// fds beeing removed before the next "poll()"
    pending_removal: FxHashSet<RawSocketFd>,
}

#[derive(Debug, Copy, Clone)]
struct FdEntry {
    index_in_fds: usize,
    token: usize,
}
