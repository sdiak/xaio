use rustc_hash::FxHashMap;
use std::{
    io::{Error, ErrorKind, Result},
    sync::{Arc, Mutex},
};

use crate::{catch_enomem, PollFlag};

use super::{poll::PollFd, PollEvent, RawSd, SelectorEvent, SelectorIFace};

pub struct Selector(Arc<Mutex<Inner>>);

impl Selector {
    pub fn try_new(initial_capacity: usize) -> Result<Self> {
        let inner = Inner::try_new(initial_capacity)?;
        Ok(catch_enomem(|| Self(Arc::new(Mutex::new(inner))))?)
    }
}

#[derive(Debug)]
struct Inner {
    pollfds: Vec<PollFd>,
    entries: FxHashMap<RawSd, FdEntry>,
}
#[derive(Debug, Copy, Clone)]
struct FdEntry {
    index_in_fds: usize,
    token: usize,
}

impl Inner {
    fn try_new(mut initial_capacity: usize) -> Result<Self> {
        if initial_capacity < 64 {
            initial_capacity = 64;
        }
        let mut entries = FxHashMap::<RawSd, FdEntry>::with_hasher(rustc_hash::FxBuildHasher);
        let mut pollfds = Vec::<PollFd>::new();
        if entries.try_reserve((initial_capacity / 2) * 3).is_err()
            || pollfds.try_reserve(initial_capacity).is_err()
        {
            Err(Error::from(ErrorKind::OutOfMemory))
        } else {
            Ok(Inner { pollfds, entries })
        }
    }
    fn register(&mut self, fd: RawSd, token: usize, interests: PollFlag) -> Result<()> {
        if !super::raw_fd_is_valid(fd) {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        // Do not register twice
        if self.entries.contains_key(&fd) {
            return Err(Error::from(ErrorKind::AlreadyExists));
        }
        if self.entries.try_reserve(1).is_err() || self.pollfds.try_reserve(1).is_err() {
            return Err(Error::from(ErrorKind::OutOfMemory));
        }
        // Adds the pollfds slot
        let index = self.pollfds.len();
        self.pollfds.push(PollFd::from_interests(fd, interests));
        // Adds the index slot
        self.entries.insert(
            fd,
            FdEntry {
                index_in_fds: index,
                token: token,
            },
        );
        Ok(())
    }
    fn reregister(&mut self, fd: RawSd, token: usize, interests: PollFlag) -> Result<()> {
        // Find the entry
        if let Some(entry) = self.entries.get_mut(&fd) {
            entry.token = token;
            self.pollfds[entry.index_in_fds].set_interests(interests);
            Ok(())
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    }
    fn unregister(&mut self, fd: RawSd) -> Result<()> {
        // Find the entry
        if let Some(entry) = self.entries.remove(&fd) {
            // Grab the pollfds cell and remove
            self.pollfds[entry.index_in_fds].disable();
            Ok(())
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    }
    fn select(&self, events: &mut Vec<SelectorEvent>, timeout_ms: i32) -> Result<()> {}
    // fn wake(&self);
}
