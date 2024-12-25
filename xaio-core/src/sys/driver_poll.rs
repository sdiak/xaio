use std::{
    io::{Error, ErrorKind, Result},
    mem::offset_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
};

use crate::{
    collection::{SLink, SList, SListNode},
    IoReq,
};

use super::{poll::PollFd, PollEvent, RawSd};

pub struct PollSocketInner {
    /// The socket
    socket: socket2::Socket,
    index_in_driver: u32,
    driver: Option<&'static Mutex<Inner>>,
    watchers: SList<IoReq>,
}
impl Drop for PollSocketInner {
    fn drop(&mut self) {
        todo!(); // TODO: remove socket + cancel ops
    }
}

pub struct PollDriver(Arc<Inner>);

impl PollDriver {
    // pub(crate) fn bind_socket(&self, socket: &mut PollSocketInner) -> Result<()> {
    //     if socket.driver.is_some() {
    //         return Err(Error::from(ErrorKind::InvalidInput));
    //     }
    //     if let Ok(mut inner) = self.0.try_lock() {
    //         return inner.bind_socket(socket);
    //     }
    //     Ok(())
    // }
    fn mutator_lock<F: FnOnce(&mut PollFds) -> Result<()>>(&self, mutate: F) -> Result<()> {
        // Notify the poller thread that there is at least one mutator
        let mut mutators_count = self.0.mutators_count.lock().expect("Unrecoverable error");
        *mutators_count += 1;
        if *mutators_count == 1 {
            // I'm the first thread waiting to mutate the poll fds, stop the polling thread
            self.0.wake();
        }
        // Grab the pollds lock and perform mutation
        let result = {
            let mut guard = self.0.pollfds.lock().expect("Unrecoverable error");
            mutate(&mut *guard)
        };
        // Released the poll fds lock
        if *mutators_count == 0 {
            // I'm the laster thread that mutated the poll fds, resume the polling thread
            self.0.mutators_cnd.notify_one();
        }
        result
    }
    fn poll_thread(&self) {
        loop {
            // Synchronize with mutators : wait for no mutator
            let mut mutators_count = self.0.mutators_count.lock().expect("Unrecoverable error");
            while *mutators_count > 0 {
                mutators_count = self
                    .0
                    .mutators_cnd
                    .wait(mutators_count)
                    .expect("Unrecoverable error");
            }
            // Grab the lock
            let pollfds = self.0.pollfds.lock().expect("Unrecoverable error");
            // then release the mutator lock
            drop(mutators_count);

            todo!("Work")
        }
    }
    // fn poll(&mut self) {
    //     let inner = self.0.lock().expect("Unrecoverable error");
    //     inner.is_polling.store(true, Ordering::Relaxed);
    //     todo!(); // TODO: Poll
    // }
}

#[derive(Debug)]
struct PollFds {
    pollfds: Vec<PollFd>,
}

#[derive(Debug)]
struct Inner {
    /// One thread can lock this mutex **only** after grabbing `mutators_count` to avoid deadlock
    pollfds: Mutex<PollFds>,
    mutators_count: Mutex<usize>,
    /// Poller thread waits on this condition until `mutators_count==0`
    mutators_cnd: Condvar,
}

impl Inner {
    // pub(crate) fn bind_socket(&mut self, socket: &mut PollSocketInner) -> Result<()> {
    //     if self.pollfds.try_reserve(1).is_err() {
    //         return Err(Error::from(ErrorKind::OutOfMemory));
    //     }
    //     Ok(())
    // }
    fn wake(&self) {
        todo!()
    }
}

#[derive(Debug, Copy, Clone)]
struct FdEntry {
    index_in_fds: usize,
    token: usize,
}
