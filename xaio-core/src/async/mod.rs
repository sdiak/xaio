use std::{
    mem::ManuallyDrop,
    ops::DerefMut,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

mod deadline;
pub use deadline::AsyncDeadline;
mod socket;
pub use socket::*;
mod file;
pub use file::*;

use crate::Status;

pub struct Driver {}
pub struct CompletionPort2 {
    now: AtomicU64,
}
impl CompletionPort2 {
    pub fn now(&self) -> u64 {
        self.now.load(Ordering::Relaxed)
    }
}

pub struct PollContext<'a> {
    now: u64,
    driver: &'a Driver,
    port: &'a CompletionPort2,
}

pub type Completion<D: AsyncOp> = fn(D, Status);

pub trait AsyncOp {
    fn poll(&mut self, cx: &PollContext) -> Status;
}

#[repr(u8)]
pub enum AsyncOpCode {
    NO_OP,
    DEADLINE,
    SEND,
    RECV,
    READ,
    WRITE,
}
impl From<u8> for AsyncOpCode {
    #[inline(always)]
    fn from(value: u8) -> Self {
        if value <= AsyncOpCode::NO_OP as u8 {
            unsafe { std::mem::transmute::<u8, AsyncOpCode>(value) }
        } else {
            AsyncOpCode::NO_OP
        }
    }
}
impl From<AsyncOpCode> for u8 {
    #[inline(always)]
    fn from(value: AsyncOpCode) -> Self {
        value as _
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AsyncNoOp {}
impl AsyncOp for AsyncNoOp {
    fn poll(&mut self, _cx: &PollContext) -> Status {
        Status::new(0)
    }
}

#[repr(C)]
union AsyncUnion {
    no_op: AsyncNoOp,
    deadline: AsyncDeadline,
    recv: ManuallyDrop<AsyncRecv>,
    send: ManuallyDrop<AsyncSend>,
    read: ManuallyDrop<AsyncRead>,
    write: ManuallyDrop<AsyncWrite>,
}

#[repr(C)]
pub struct Async {
    status: AtomicI32,
    flags_and_op_code: u32,
    concrete: AsyncUnion,
}
impl Async {
    #[inline(always)]
    pub const fn raw_code(&self) -> u8 {
        (self.flags_and_op_code & 0xFFu32) as u8
    }
    #[inline(always)]
    const unsafe fn code_unchecked(&self) -> AsyncOpCode {
        unsafe { std::mem::transmute::<u8, AsyncOpCode>(self.raw_code()) }
    }
    #[inline]
    pub const fn code(&self) -> AsyncOpCode {
        let value = self.raw_code();
        if value <= AsyncOpCode::NO_OP as u8 {
            unsafe { std::mem::transmute::<u8, AsyncOpCode>(value) }
        } else {
            AsyncOpCode::NO_OP
        }
    }
}
impl AsyncOp for Async {
    fn poll(&mut self, cx: &PollContext) -> Status {
        unsafe {
            match self.code_unchecked() {
                AsyncOpCode::NO_OP => self.concrete.no_op.poll(cx),
                AsyncOpCode::DEADLINE => self.concrete.deadline.poll(cx),
                AsyncOpCode::RECV => self.concrete.recv.deref_mut().poll(cx),
                AsyncOpCode::SEND => self.concrete.send.deref_mut().poll(cx),
                AsyncOpCode::READ => self.concrete.read.deref_mut().poll(cx),
                AsyncOpCode::WRITE => self.concrete.write.deref_mut().poll(cx),
            }
        }
    }
}
impl Drop for Async {
    fn drop(&mut self) {
        unsafe {
            match self.code_unchecked() {
                AsyncOpCode::RECV => ManuallyDrop::drop(&mut self.concrete.recv),
                AsyncOpCode::SEND => ManuallyDrop::drop(&mut self.concrete.send),
                AsyncOpCode::READ => ManuallyDrop::drop(&mut self.concrete.read),
                AsyncOpCode::WRITE => ManuallyDrop::drop(&mut self.concrete.write),
                _ => (),
            }
        }
    }
}
