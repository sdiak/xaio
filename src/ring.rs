use std::{cell::RefCell, rc::Rc};

use crate::{PhantomUnsend, PhantomUnsync, Request};
use std::io::{Error, ErrorKind, Result};

pub(crate) struct RingInner {
    ref_count: usize,
    _unsync: PhantomUnsync,
    _unsend: PhantomUnsend,
}

pub struct Ring {
    inner: RefCell<RingInner>,
}

#[repr(transparent)]
pub struct Completion {
    inner: Request,
}

impl Drop for Completion {
    fn drop(&mut self) {
        let ring = self.inner.owner.borrow_mut();

        // if let Some(&mut ring) = self.inner.owner {}
    }
}

impl Ring {
    pub fn submit<'a, 'b>(&'a self, sub: &'b mut Request) -> Result<&'b mut Completion> {
        // todo!(); TODO:
        Ok(unsafe { std::mem::transmute(sub) })
    }
}

impl RingInner {
    fn new() -> Self {
        Self {
            ref_count: 1,
            _unsync: PhantomUnsync {},
            _unsend: PhantomUnsend {},
        }
    }
    fn cancel(sub: &Completion) {}
}
