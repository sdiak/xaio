use std::{cell::RefCell, rc::Rc};

use crate::{
    request_queue::RequestQueue, request_queue::RequestQueueParkScope, PhantomUnsend,
    PhantomUnsync, ReadyList, Request,
};
use std::io::{Error, ErrorKind, Result};

pub(crate) struct RingInner {
    ref_count: usize,
    concurrent: RequestQueue,
    ready: ReadyList,
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
            concurrent: RequestQueue::new(),
            ready: ReadyList::new(),
            _unsync: PhantomUnsync {},
            _unsend: PhantomUnsend {},
        }
    }
    fn cancel(sub: &Completion) {}
    pub fn wait(&mut self) {
        let need_park: bool = false;
        if need_park {
            let _scoped_parker =
                RequestQueueParkScope::new(&mut self.concurrent, &mut self.ready, need_park);
            todo!();
        }
    }
}
