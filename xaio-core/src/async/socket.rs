use std::{io::ErrorKind, mem::MaybeUninit};

use crate::Status;

use super::{AsyncData, PollContext};

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
const _MSG_DONTWAIT: libc::c_int = libc::MSG_DONTWAIT as _;
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
const _MSG_DONTWAIT: libc::c_int = 0 as _;

pub struct SendData {
    pub socket: socket2::Socket, // TODO: driver socket ?
    pub buffer: crate::IoBuf,
    pub todo: i32,
    pub done: i32,
}

impl AsyncData for SendData {
    fn poll(&mut self, _cx: &PollContext) -> Status {
        while self.todo < self.done {
            match self.socket.send_with_flags(
                &self.buffer.as_slice()[self.done as usize..self.todo as usize],
                _MSG_DONTWAIT,
            ) {
                Ok(len) => {
                    if len == 0 {
                        return Status::new(self.done);
                    }
                    self.done += len as i32;
                }
                Err(err) => match err.kind() {
                    ErrorKind::Interrupted => {}
                    ErrorKind::WouldBlock => {
                        return Status::pending();
                    }
                    _ => return Status::from(err),
                },
            }
        }
        Status::new(self.done)
    }
}

pub struct RecvData {
    pub socket: socket2::Socket, // TODO: driver socket ?
    pub buffer: crate::IoBuf,
    pub todo: i32,
    pub done: i32,
}

impl AsyncData for RecvData {
    fn poll(&mut self, _cx: &PollContext) -> Status {
        while self.todo < self.done {
            match self.socket.recv_with_flags(
                unsafe {
                    std::mem::transmute::<&mut [u8], &mut [MaybeUninit<u8>]>(
                        &mut self.buffer.as_slice_mut()[self.done as usize..self.todo as usize],
                    )
                },
                _MSG_DONTWAIT,
            ) {
                Ok(len) => {
                    if len == 0 {
                        return Status::new(self.done);
                    }
                    self.done += len as i32;
                }
                Err(err) => match err.kind() {
                    ErrorKind::Interrupted => {}
                    ErrorKind::WouldBlock => {
                        return Status::pending();
                    }
                    _ => return Status::from(err),
                },
            }
        }
        Status::new(self.done)
    }
}
