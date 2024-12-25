use std::{
    io::{Error, ErrorKind, Result},
    mem::offset_of,
    os::fd::{AsRawFd, OwnedFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    task::Poll,
    thread::JoinHandle,
};

use crate::{
    collection::{SLink, SList, SListNode},
    io_driver::IoDriverConfig,
    sys::raw_sd_is_valid,
    IoReq, OpCode, OpCodeSet, PollFlag,
};

use super::ioutils::{pipe2, write_all};
use super::{poll::PollFd, PollEvent, RawSd};

pub struct PollSocketInner {
    /// The socket
    socket: socket2::Socket,
    index_in_driver: u32,
    driver: Option<&'static Mutex<Inner>>,
    watchers: SList<IoReq>,
}
fn compute_interests(watchers: &SList<IoReq>) -> PollFlag {
    let mut flags = PollFlag::empty();
    for watcher in watchers.iter() {
        flags |= unsafe { watcher.op_data.socket.events };
        if flags.contains(PollFlag::READABLE | PollFlag::WRITABLE | PollFlag::PRIORITY) {
            return flags;
        }
    }
    flags
}
impl Drop for PollSocketInner {
    fn drop(&mut self) {
        todo!(); // TODO: remove socket + cancel ops
    }
}

#[derive(Debug, Clone)]
pub struct PollDriver(Arc<Inner>);

impl PollDriver {
    pub const SUPPORTED_OP_CODES: OpCodeSet =
        OpCodeSet::new(&[OpCode::POLL_CTL_ADD, OpCode::POLL_CTL_DEL]);

    #[inline(always)]
    fn supported_op_codes() -> &'static OpCodeSet {
        &PollDriver::SUPPORTED_OP_CODES
    }

    pub fn new(config: &IoDriverConfig) -> Result<Self> {
        let inner = Inner::new(config.max_number_of_fd_hint)?;
        let inner = crate::catch_enomem(|| Arc::new(inner))?;
        // let thiz = Self(crate::catch_enomem(|| Arc::new(inner))?);
        let inner_4_poll_thread = inner.clone();
        let _ = std::thread::spawn(move || inner_4_poll_thread.poll_thread());
        Ok(Self(inner))
    }
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
        self.0.mutator_lock(mutate)
    }
    pub fn wake(&self) {
        self.0.wake();
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
    len: usize,
}
impl PollFds {
    fn new(mut initial_capacity: usize, waker: RawSd) -> Result<Self> {
        if initial_capacity < 64 {
            initial_capacity = 64;
        }
        let mut pollfds = Vec::<PollFd>::new();
        if pollfds.try_reserve(initial_capacity).is_err() {
            Err(Error::from(ErrorKind::OutOfMemory))
        } else {
            pollfds.push(PollFd::new(waker, PollEvent::IN));
            Ok(Self { pollfds, len: 1 })
        }
    }

    fn add(&mut self, socket: &mut PollSocketInner) -> Result<()> {
        debug_assert!(raw_sd_is_valid(socket.socket.as_raw_fd() as _));
        let interests = crate::sys::interests_to_events(compute_interests(&socket.watchers));
        if self.len < self.pollfds.len() {
            // There is a free slot
            for i in 1..self.pollfds.len() {
                if !raw_sd_is_valid(self.pollfds[i].fd() as _) {
                    self.pollfds[i] = PollFd::new(socket.socket.as_raw_fd() as _, interests);
                    self.len += 1;
                    socket.index_in_driver = i as _;
                    return Ok(());
                }
            }
            std::unreachable!("There must be a free slot");
        } else if (self.pollfds.len() < u32::MAX as _) && self.pollfds.try_reserve(1).is_ok() {
            socket.index_in_driver = self.pollfds.len() as _;
            self.pollfds
                .push(PollFd::new(socket.socket.as_raw_fd() as _, interests));
            self.len += 1;
            Ok(())
        } else {
            Err(Error::from(ErrorKind::OutOfMemory))
        }
    }
}

#[derive(Debug)]
struct Inner {
    /// One thread can lock this mutex **only** after grabbing `mutators_count` to avoid deadlock
    pollfds: Mutex<PollFds>,
    mutators_count: Mutex<usize>,
    /// Poller thread waits on this condition until `mutators_count==0`
    mutators_cnd: Condvar,
    /// Waker pipe
    waker: (OwnedFd, OwnedFd),
    /// Used configuration
    config: IoDriverConfig,
}

impl Inner {
    fn new(initial_capacity: u32) -> Result<Self> {
        let mut config = IoDriverConfig::zeroed();
        config.max_number_of_fd_hint = num::clamp(initial_capacity, 64, 1 << 20);
        let waker = pipe2(true, true)?;
        let pollfds = PollFds::new(config.max_number_of_fd_hint as _, waker.0.as_raw_fd())?;
        Ok(Self {
            pollfds: Mutex::new(pollfds),
            mutators_count: Mutex::new(0),
            mutators_cnd: Condvar::new(),
            waker,
            config,
        })
    }
    // pub(crate) fn bind_socket(&mut self, socket: &mut PollSocketInner) -> Result<()> {
    //     if self.pollfds.try_reserve(1).is_err() {
    //         return Err(Error::from(ErrorKind::OutOfMemory));
    //     }
    //     Ok(())
    // }
    fn wake(&self) {
        let buf = [1u8];
        write_all(self.waker.1.as_raw_fd(), &buf, true).expect("Unrecoverable error");
    }
    fn mutator_lock<F: FnOnce(&mut PollFds) -> Result<()>>(&self, mutate: F) -> Result<()> {
        // Notify the poller thread that there is at least one mutator
        let mut mutators_count = self.mutators_count.lock().expect("Unrecoverable error");
        *mutators_count += 1;
        if *mutators_count == 1 {
            // I'm the first thread waiting to mutate the poll fds, stop the polling thread
            self.wake();
        }
        // Grab the pollds lock and perform mutation
        let result = {
            let mut guard = self.pollfds.lock().expect("Unrecoverable error");
            mutate(&mut *guard)
        };
        // Released the poll fds lock
        if *mutators_count == 0 {
            // I'm the laster thread that mutated the poll fds, resume the polling thread
            self.mutators_cnd.notify_one();
        }
        result
    }
    fn poll_thread(&self) {
        // TODO: catch end of life and exit
        loop {
            // Synchronize with mutators : wait for no mutator
            let mut mutators_count = self.mutators_count.lock().expect("Unrecoverable error");
            while *mutators_count > 0 {
                mutators_count = self
                    .mutators_cnd
                    .wait(mutators_count)
                    .expect("Unrecoverable error");
            }
            // Grab the lock
            let pollfds = self.pollfds.lock().expect("Unrecoverable error");
            // then release the mutator lock
            drop(mutators_count);

            todo!("Work")
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct FdEntry {
    index_in_fds: usize,
    token: usize,
}
