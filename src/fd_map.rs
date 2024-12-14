use std::{
    io::{Error, ErrorKind, Result},
    mem::{ManuallyDrop, MaybeUninit},
    ops::Deref,
    ptr::NonNull,
};

use rustc_hash::{FxBuildHasher, FxHashMap};
use socket2::Socket;

use crate::{request, selector::Interest, ReadyList, Request, RequestList};

#[cfg(not(target_os = "windows"))]
type Fd = libc::c_int;
#[cfg(target_os = "windows")]
type Fd = usize;

pub(crate) struct FdMap {
    entries: FxHashMap<Fd, Entry>,
}

struct Entry {
    reader: Option<NonNull<Request>>,
    writer: Option<NonNull<Request>>,
}

impl FdMap {
    pub(crate) fn new(capacity: usize) -> Result<Self> {
        match std::panic::catch_unwind(|| {
            FxHashMap::<Fd, Entry>::with_capacity_and_hasher(capacity, FxBuildHasher)
        }) {
            Ok(entries) => Ok(Self { entries }),
            Err(_) => Err(Error::from(ErrorKind::OutOfMemory)),
        }
    }
    pub(crate) fn update(
        &mut self,
        fd: Fd,
        reader: Option<NonNull<Request>>,
        writer: Option<NonNull<Request>>,
    ) -> std::io::Result<()> {
        if let Some(entry) = self.entries.get_mut(&fd) {
            // Check for a single reader and a single writer
            if (entry.reader.is_some() && reader.is_some())
                || (entry.writer.is_some() && writer.is_some())
            {
                return Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy));
            }
            entry.reader = reader;
            entry.writer = writer;
            // Drop the entry if there are no more watchers
            if entry.reader.is_none() && entry.writer.is_none() {
                self.entries.remove(&fd);
            }
        } else if reader.is_some() || writer.is_some() {
            self.entries.try_reserve(1)?;
            self.entries
                .insert(fd, Entry { reader, writer })
                .expect("Memory is reserved");
        }
        Ok(())
    }
}

struct PendingOps {
    pending: RequestList,
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
const _MSG_DONTWAIT: libc::c_int = libc::MSG_DONTWAIT as _;
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
const _MSG_DONTWAIT: libc::c_int = 0 as _;

/// Perform the given operation `f`
/// # Returns
/// `true` when the request should be polled again
fn _socket_req_resume<F>(req: &mut Request, poll_events: &mut i32, events: &mut i32, f: F) -> bool
where
    F: FnOnce(&Socket, &mut [u8], &mut i32) -> i32,
{
    let socket = req.get_socket();
    let buffer = unsafe {
        std::slice::from_raw_parts_mut::<u8>(
            req.op.socket.buffer.offset(req.op.socket.done as _),
            (req.op.socket.todo - req.op.socket.done) as _,
        )
    };

    let status = f(ManuallyDrop::deref(&socket), buffer, events);
    if (status >= 0) {
        unsafe { req.op.socket.done += status as u32 };
        if unsafe { req.op.socket.done >= req.op.socket.todo } {
            // DONE: return the request to user
            req.set_status_local(unsafe { req.op.socket.done } as _);
            false
        } else {
            // Still some work to do
            true
        }
    } else {
        // No more data to read on the socket
        *events &= !Interest::READABLE.bits() as i32;
        // No more data to write on the socket
        *events &= !Interest::WRITABLE.bits() as i32;
        // Socket has error
        *events |= Interest::ERROR.bits() as i32;
        *poll_events |= Interest::ERROR.bits() as i32;
        // DONE: return the request to user
        req.set_status_local(status);
        false
    }
}

fn _socket_req_resume_recv(socket: &Socket, buffer: &mut [u8], events: &mut i32) -> i32 {
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
                    *events &= !Interest::READABLE.bits() as i32;
                    return 0;
                }
                _ => {
                    return -crate::utils::io_error_to_errno_constant(&err);
                }
            },
        }
    }
}
fn _socket_req_resume_send(socket: &Socket, buffer: &mut [u8], events: &mut i32) -> i32 {
    loop {
        match socket.send_with_flags(buffer, _MSG_DONTWAIT) {
            Ok(len) => {
                return len as i32;
            }
            Err(err) => match err.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::WouldBlock => {
                    // No more data to read on the socket
                    *events &= !Interest::WRITABLE.bits() as i32;
                    return 0;
                }
                _ => {
                    return -crate::utils::io_error_to_errno_constant(&err);
                }
            },
        }
    }
}

impl PendingOps {
    pub(crate) fn add(&mut self, req: NonNull<Request>) {
        unsafe {
            assert!(req.as_ref().is_a_socket_op(), "Not a file/socket operation");
            // FIFO Order but poll operation comes first
            // self.pending.insert_sorted::<PendingOps>(req);
            self.pending.push_back2(req);
        }
    }
    pub(crate) fn process_event(&mut self, mut events: i32, ready: &mut ReadyList) {
        debug_assert!(!self.pending.is_empty());
        let mut poll_events = events;
        ready.push_back_all(&mut self.pending.retain_mut(|req| {
            let is_poll_op = req.opcode_raw() == request::OP_SOCKET_POLL;
            let interests = unsafe { req.op.socket.interests } as i32;
            let op_poll_events = interests & poll_events;
            // Safety all socket op share the same union field
            if !is_poll_op && (interests & events) != 0 {
                _socket_req_resume(
                    req,
                    &mut poll_events,
                    &mut events,
                    match req.opcode_raw() {
                        request::OP_SOCKET_RECV => _socket_req_resume_recv,
                        request::OP_SOCKET_SEND => _socket_req_resume_send,
                        _ => {
                            panic!("Unknown operation type : {:?}", req.opcode());
                        }
                    },
                )
            } else if is_poll_op && op_poll_events != 0 {
                if (interests & Interest::ONESHOT.bits() as i32) != 0 {
                    // Success one-shot, set the status and returns to user
                    req.set_status_local(op_poll_events);
                    false
                } else if !ready.alloc_and_pushback(req, op_poll_events) {
                    // Failed to allocate a multishot event, return the registration with an error
                    req.set_status_local(-libc::ENOMEM);
                    false
                } else {
                    // Multi-shot "instance" pushed to `ready`, keep the registration
                    true
                }
            } else {
                // Not watching the events, keep the registration
                true
            }
        }));
    }
}
