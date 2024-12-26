use std::{
    io::{Error, ErrorKind, Result},
    mem::{offset_of, MaybeUninit},
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

use super::{
    io_error_to_errno_constant,
    ioutils::{pipe2, write_all},
};
use super::{poll::PollFd, PollEvent, RawSd};

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
const _MSG_DONTWAIT: libc::c_int = libc::MSG_DONTWAIT as _;
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
const _MSG_DONTWAIT: libc::c_int = 0 as _;

pub struct PollSocketInner {
    /// The socket
    socket: socket2::Socket,
    index_in_driver: u32,
    driver: Option<&'static Mutex<Inner>>,
    watchers: SList<IoReq>,
}
impl PollSocketInner {
    pub(crate) fn compute_interests(&self) -> PollFlag {
        let all_events = PollFlag::READABLE | PollFlag::WRITABLE | PollFlag::PRIORITY;
        let mut flags = PollFlag::empty();
        for watcher in self.watchers.iter() {
            flags |= unsafe { watcher.op_data.socket.events };
            if flags.contains(all_events) {
                return flags;
            }
        }
        flags
    }
    pub(crate) fn process_events(&mut self, mut events: PollFlag) {
        let mut poll_events = events;
        let mut done = self.watchers.retain(|req| match req.opcode() {
            OpCode::RECV => _socket_req_resume(
                &self.socket,
                req,
                &mut poll_events,
                &mut events,
                _socket_req_resume_recv,
            ),
            OpCode::SEND => _socket_req_resume(
                &self.socket,
                req,
                &mut poll_events,
                &mut events,
                _socket_req_resume_send,
            ),
            _ => {
                req._set_status(-libc::ENOSYS);
                false
            }
        });
        // TODO: batch as a list per port ?
        while let Some(req) = done.pop_front() {
            req.completion_port()._send_completed(req);
        }
    }
}

/// Perform the given operation `f`
/// # Returns
/// `true` when the request should be polled again
fn _socket_req_resume<F>(
    socket: &socket2::Socket,
    req: &mut IoReq,
    poll_events: &mut PollFlag,
    events: &mut PollFlag,
    f: F,
) -> bool
where
    F: FnOnce(&socket2::Socket, &mut [u8], &mut PollFlag) -> i32,
{
    let op = unsafe { &mut *req.op_data.socket };
    let buffer = op.buffer_mut();

    let status = f(socket, buffer, events);
    if status >= 0 {
        op.done += status as u32;
        if op.done >= op.todo {
            // DONE: return the request to user
            let status = op.done;
            drop(op);
            req._set_status(status as i32);
            false
        } else {
            // Still some work to do
            true
        }
    } else {
        // No more data to read on the socket
        *events &= !PollFlag::READABLE;
        // No more data to write on the socket
        *events &= !PollFlag::WRITABLE;
        // Socket has error
        *events |= PollFlag::ERROR;
        *poll_events |= PollFlag::ERROR;
        // DONE: return the request to user
        req._set_status(status);
        false
    }
}

fn _socket_req_resume_recv(
    socket: &socket2::Socket,
    buffer: &mut [u8],
    events: &mut PollFlag,
) -> i32 {
    let buffer = unsafe { std::mem::transmute::<&mut [u8], &mut [MaybeUninit<u8>]>(buffer) };
    loop {
        match socket.recv_with_flags(buffer, _MSG_DONTWAIT) {
            Ok(len) => {
                return len as i32;
            }
            Err(err) => match err.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::WouldBlock => {
                    // No more data to read on the socket
                    *events &= !PollFlag::READABLE;
                    return 0;
                }
                _ => {
                    return -io_error_to_errno_constant(&err);
                }
            },
        }
    }
}
fn _socket_req_resume_send(
    socket: &socket2::Socket,
    buffer: &mut [u8],
    events: &mut PollFlag,
) -> i32 {
    loop {
        match socket.send_with_flags(buffer, _MSG_DONTWAIT) {
            Ok(len) => {
                return len as i32;
            }
            Err(err) => match err.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::WouldBlock => {
                    // No more data to read on the socket
                    *events &= !PollFlag::WRITABLE;
                    return 0;
                }
                _ => {
                    return -io_error_to_errno_constant(&err);
                }
            },
        }
    }
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
    pub const fn supported_op_codes() -> &'static OpCodeSet {
        &PollDriver::SUPPORTED_OP_CODES
    }

    pub fn new(config: &IoDriverConfig) -> Result<Self> {
        let inner = Inner::new(config.max_number_of_fd_hint)?;
        let inner = crate::catch_enomem(|| Arc::new(inner))?;
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
    // fn mutator_lock<F: FnOnce(&mut PollFds) -> Result<()>>(&self, mutate: F) -> Result<()> {
    //     self.0.mutator_lock(mutate)
    // }
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
        let interests = crate::sys::interests_to_events(socket.compute_interests());
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
            let mut pollfds = self.pollfds.lock().expect("Unrecoverable error");
            // then release the mutator lock
            drop(mutators_count);

            match super::poll::poll(&mut pollfds.pollfds, -1) {
                Ok(mut nevents) => {
                    let mut i = 0;
                    while nevents > 0 {
                        if let Some(revents) = pollfds.pollfds[i].rinterests() {
                            todo!("HAndle event");
                            nevents -= 1;
                        }
                        i += 1;
                    }
                }
                Err(e) => {
                    eprintln!("libc::poll(...) failed: {}\nAborting ...", e);
                    std::process::abort();
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct FdEntry {
    index_in_fds: usize,
    token: usize,
}
