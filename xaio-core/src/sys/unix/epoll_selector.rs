use std::{
    io::{Error, ErrorKind},
    ops::Deref,
    sync::Arc,
};

use crate::{sys::epoll::Event, IoReq, PollFlag};
use std::io::Result;

use super::epoll::{EPoll, EPollEvent};

pub struct selector(Arc<Inner>);

const _: () = assert!(
    std::mem::align_of::<EPollEvent>() >= std::mem::align_of::<libc::epoll_event>()
        && std::mem::size_of::<EPollEvent>() == std::mem::size_of::<libc::epoll_event>()
        && PollFlag::READABLE.bits() == libc::EPOLLIN as u16
        && PollFlag::WRITABLE.bits() == libc::EPOLLOUT as u16
        && PollFlag::PRIORITY.bits() == libc::EPOLLPRI as u16
        && PollFlag::ERROR.bits() == libc::EPOLLERR as u16
        && PollFlag::HANG_UP.bits() == libc::EPOLLHUP as u16
        && PollFlag::RDHANG_UP.bits() == libc::EPOLLRDHUP as u16
);

struct Inner {
    epoll: EPoll,
}

impl Inner {
    pub fn submit(&self, req: Box<IoReq>) {
        if !req.is_a_socket_op() {
            req._complete(-libc::ENOSYS);
        } else {
            let sd = unsafe { &*req.op_data.socket.deref() };
            let mut events = (sd.interests & !PollFlag::ONESHOT).bits() as libc::c_int;
            if sd.interests.contains(PollFlag::ONESHOT) {
                events |= libc::EPOLLONESHOT | libc::EPOLLET;
            }
            if let Err(e) = self.epoll.ctl(
                sd.socket,
                libc::EPOLL_CTL_ADD,
                Event::from_bits_truncate(events),
                req.as_ref() as *const IoReq as usize as _,
            ) {
                req._complete(IoReq::STATUS_OTHER); // TODO: error conversion
            } else {
                std::mem::forget(req);
            }
        }
    }
}
