use std::{cell::RefCell, marker::PhantomData, ptr::NonNull, rc::Rc, sync::atomic::AtomicU32};

use crate::{
    driver_waker::DriverWaker,
    request_queue::{RequestQueue, RequestQueueParkScope},
    Driver, PhantomUnsend, PhantomUnsync, ReadyList, Request,
};
use std::io::{Error, ErrorKind, Result};

pub(crate) struct RingInner {
    rc: u32,
    arc: AtomicU32, // TODO: prefer counting the wakers
    driver: Box<Driver>,
    concurrent: RequestQueue,
    ready: ReadyList,
    _unsync: PhantomUnsync,
    _unsend: PhantomUnsend,
}

pub struct Ring {
    inner: Box<RingInner>,
}
impl Drop for Ring {
    fn drop(&mut self) {
        if self.inner.rc > 1 || self.inner.arc.load(std::sync::atomic::Ordering::Relaxed) != 0 {
            log::warn!("Need to cancel everything and wait"); // TODO:
        }
    }
}
impl Ring {
    pub fn new(driver: Box<Driver>) -> Self {
        Self {
            inner: Box::new(RingInner::new(driver)),
        }
    }
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
    fn new(driver: Box<Driver>) -> Self {
        Self {
            rc: 1 as _,
            arc: AtomicU32::new(0u32),
            driver: driver,
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
