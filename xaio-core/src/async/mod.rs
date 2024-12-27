use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::DerefMut,
    ptr::NonNull,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

mod deadline;
pub use deadline::AsyncDeadline;
mod socket;
pub use socket::*;
mod file;
pub use file::*;

use crate::{IoBuf, Socket, Status};

pub struct Driver {}
pub struct CompletionPort2 {
    now: AtomicU64,
}
impl CompletionPort2 {
    pub fn now(&self) -> u64 {
        self.now.load(Ordering::Relaxed)
    }

    pub fn set_deadline(&self, deadline: u64) -> Option<Handle> {
        if let Some((op, handle)) = Async::new(AsyncDeadline::new(deadline)) {
            std::mem::forget(op); // TODO:
            Some(handle)
        } else {
            None
        }
    }
}

pub struct PollContext<'a> {
    now: u64,
    driver: &'a Driver,
    port: &'a CompletionPort2,
}

pub type Completion<O: AsyncOp> = fn(O, Status);

pub trait AsyncOp {
    const OP_CODE: AsyncOpCode;
    fn poll(&mut self, cx: &PollContext) -> Status;
}

#[repr(transparent)]
pub struct Handle {
    promise: NonNull<AsyncInner>,
}
impl Drop for Handle {
    fn drop(&mut self) {
        let inner = unsafe { self.promise.as_mut() };
        if let Err(_) = inner.status.compare_exchange(
            Status::PENDING,
            -libc::ECANCELED,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            // Sole owner, drop
            let _ = *inner;
        }
    }
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
    const OP_CODE: AsyncOpCode = AsyncOpCode::NO_OP;
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
pub struct Async(Box<AsyncInner>);

struct AsyncInner {
    status: AtomicI32,
    flags_and_op_code: u32,
    concrete: AsyncUnion,
}
impl AsyncInner {
    fn new<O: AsyncOp>(op: O) -> Self {
        let mut thiz = Self {
            status: AtomicI32::new(Status::PENDING),
            flags_and_op_code: O::OP_CODE as u8 as _,
            ..unsafe { std::mem::zeroed() }
        };
        unsafe { std::ptr::write(&mut thiz.concrete as *mut AsyncUnion as *mut O, op) };
        thiz
    }
}
impl Async {
    const LAYOUT: std::alloc::Layout = unsafe {
        std::alloc::Layout::from_size_align_unchecked(
            std::mem::size_of::<AsyncInner>(),
            std::mem::align_of::<AsyncInner>(),
        )
    };
    fn new<O: AsyncOp>(op: O) -> Option<(Self, Handle)> {
        let inner = unsafe { std::alloc::alloc(Async::LAYOUT) } as *mut AsyncInner;
        if !inner.is_null() {
            unsafe { std::ptr::write(inner, AsyncInner::new(op)) };
            Some((
                Self(unsafe { Box::from_raw(inner) }),
                Handle {
                    promise: unsafe { NonNull::new_unchecked(inner) },
                },
            ))
        } else {
            None
        }
    }
    #[inline(always)]
    pub const fn raw_code(&self) -> u8 {
        (self.0.flags_and_op_code & 0xFFu32) as u8
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
    const OP_CODE: AsyncOpCode = AsyncOpCode::NO_OP;
    fn poll(&mut self, cx: &PollContext) -> Status {
        unsafe {
            match self.code_unchecked() {
                AsyncOpCode::NO_OP => self.0.concrete.no_op.poll(cx),
                AsyncOpCode::DEADLINE => self.0.concrete.deadline.poll(cx),
                AsyncOpCode::RECV => self.0.concrete.recv.deref_mut().poll(cx),
                AsyncOpCode::SEND => self.0.concrete.send.deref_mut().poll(cx),
                AsyncOpCode::READ => self.0.concrete.read.deref_mut().poll(cx),
                AsyncOpCode::WRITE => self.0.concrete.write.deref_mut().poll(cx),
            }
        }
    }
}
impl Drop for Async {
    fn drop(&mut self) {
        unsafe {
            match self.code_unchecked() {
                AsyncOpCode::RECV => ManuallyDrop::drop(&mut self.0.concrete.recv),
                AsyncOpCode::SEND => ManuallyDrop::drop(&mut self.0.concrete.send),
                AsyncOpCode::READ => ManuallyDrop::drop(&mut self.0.concrete.read),
                AsyncOpCode::WRITE => ManuallyDrop::drop(&mut self.0.concrete.write),
                _ => (),
            }
        }
    }
}
