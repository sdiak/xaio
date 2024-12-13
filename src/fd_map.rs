use std::{
    io::{Error, ErrorKind, Result},
    mem::{ManuallyDrop, MaybeUninit},
    os::windows::io::FromRawSocket,
    ptr::NonNull,
};

use rustc_hash::{FxBuildHasher, FxHashMap};
use socket2::Socket;

use crate::{request, selector::Interest, ReadyList, Request, RequestList, RequestOrd};

// TODO: replace pin with addr

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
// impl RequestOrd for PendingOps {
//     fn before(a: &Request, b: *const Request) -> bool {
//         b.is_null()
//             || (a.opcode_raw() == request::OP_SOCKET_POLL
//                 && unsafe { (*b).opcode_raw() != request::OP_SOCKET_POLL })
//     }
// }
impl PendingOps {
    pub(crate) fn add(&mut self, req: NonNull<Request>) {
        unsafe {
            assert!(req.as_ref().is_a_socket_op(), "Not a file/socket operation");
            // FIFO Order but poll operation comes first
            // self.pending.insert_sorted::<PendingOps>(req);
            self.pending.push_back2(req);
        }
    }
    /// Apply the events to the given request and returns `true` if the request is still running
    fn apply_event(req: &mut Request, events: &mut i32) -> bool {
        #[cfg(has_libc_MSG_DONTWAIT)]
        let flags = libc::MSG_DONTWAIT;
        #[cfg(not(has_libc_MSG_DONTWAIT))]
        let flags = 0;

        let opcode = req.opcode_raw();
        let sockop = unsafe { &mut req.op.socket };
        let socket =
            ManuallyDrop::new(unsafe { Socket::from_raw_socket(sockop.socket.inner as _) });
        let rbuffer = unsafe {
            std::slice::from_raw_parts_mut::<MaybeUninit<u8>>(
                std::mem::transmute::<*mut u8, *mut MaybeUninit<u8>>(
                    sockop.buffer.offset(sockop.done as _),
                ),
                (sockop.todo - sockop.done) as _,
            )
        };
        let wbuffer = unsafe {
            std::slice::from_raw_parts_mut::<u8>(
                sockop.buffer.offset(sockop.done as _),
                (sockop.todo - sockop.done) as _,
            )
        };
        // TODO: how to handle multishot events !!!!!
        match opcode {
            request::OP_SOCKET_RECV | request::OP_SOCKET_SEND => {
                let mask = if opcode == request::OP_SOCKET_RECV {
                    !Interest::READABLE.bits()
                } else {
                    !Interest::WRITABLE.bits()
                };
                loop {
                    let status = if opcode == request::OP_SOCKET_RECV {
                        socket.recv_with_flags(rbuffer, flags)
                    } else {
                        socket.send_with_flags(wbuffer, flags)
                    };
                    match status {
                        Ok(sz) => {
                            sockop.done += sz as u32; // TODO: use concurrent_status
                            if sockop.done == sockop.todo {
                                // DONE: return the request to user
                                req.status = sockop.todo as _;
                                return false;
                            } else {
                                // No more event of the same type to try.
                                *events &= mask as i32;
                            }
                            return true;
                        }
                        Err(err) => {
                            match err.kind() {
                                ErrorKind::WouldBlock => {
                                    return true;
                                }
                                ErrorKind::Interrupted => {
                                    // Try again
                                }
                                _ => {
                                    // DONE: return the request to user
                                    req.status = -crate::utils::io_error_to_errno_constant(&err);
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                panic!("Unknown opcode");
            }
        }
        true
    }
    pub(crate) fn process_event(&mut self, mut events: i32, ready: &mut ReadyList) {
        debug_assert!(!self.pending.is_empty());
        let poll_events = events;
        ready.push_back_all(&mut self.pending.retain_mut(|req| {
            let is_poll_op = req.opcode_raw() == request::OP_SOCKET_POLL;
            let interests = unsafe { req.op.socket.interests } as i32;
            let op_poll_events = interests & poll_events;
            // Safety all socket op share the same union field
            if !is_poll_op && (interests & events) != 0 {
                PendingOps::apply_event(req, &mut events)
            } else if is_poll_op && op_poll_events != 0 {
                if (interests & Interest::ONESHOT.bits() as i32) != 0 {
                    // Success one-shot, set the status and returns to user
                    req.status = op_poll_events;
                    false
                } else if !ready.alloc_and_pushback(req, op_poll_events) {
                    // Failed to allocate a multishot event, return the registration with an error
                    req.status = -libc::ENOMEM;
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
