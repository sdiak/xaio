use std::{
    fs::File,
    io::{Read, Write},
};

use crate::Status;

use super::{AsyncOp, AsyncOpCode};

pub struct AsyncRead {
    pub file: std::fs::File, // TODO: driver fd ?
    pub buffer: crate::IoBuf,
    pub todo: i32,
    pub done: i32,
}

impl AsyncOp for AsyncRead {
    const OP_CODE: AsyncOpCode = AsyncOpCode::READ;
    fn poll(&mut self, _cx: &super::PollContext) -> crate::Status {
        match self
            .file
            .read_exact(&mut self.buffer.as_slice_mut()[self.done as usize..self.todo as usize])
        {
            Ok(_) => {
                self.done = self.todo;
                Status::new(self.done)
            }
            Err(err) => Status::from(err),
        }
    }
}

pub struct AsyncWrite {
    pub file: std::fs::File, // TODO: driver socket ?
    pub buffer: crate::IoBuf,
    pub todo: i32,
    pub done: i32,
}

impl AsyncOp for AsyncWrite {
    const OP_CODE: AsyncOpCode = AsyncOpCode::WRITE;
    fn poll(&mut self, _cx: &super::PollContext) -> crate::Status {
        match self
            .file
            .write_all(&self.buffer.as_slice()[self.done as usize..self.todo as usize])
        {
            Ok(_) => {
                self.done = self.todo;
                Status::new(self.done)
            }
            Err(err) => Status::from(err),
        }
    }
}
