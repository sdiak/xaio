use std::{cell::RefCell, marker::PhantomData, ptr::NonNull, rc::Rc, sync::atomic::AtomicU32};

use crate::{
    driver_waker::DriverWaker,
    request_queue::{RequestQueue, RequestQueueParkScope},
    Driver, PhantomUnsend, PhantomUnsync, ReadyList, Request,
};
use std::io::{Error, ErrorKind, Result};

pub(crate) struct RingInner {
    rc: u32,
    arc: AtomicU32,
    driver: Box<Driver>,
    concurrent: RequestQueue,
    ready: ReadyList,
    _unsync: PhantomUnsync,
    _unsend: PhantomUnsend,
}

pub struct Ring {
    inner: NonNull<RefCell<RingInner>>,
    _phantom: PhantomData<RefCell<RingInner>>,
}
impl Clone for Ring {
    fn clone(&self) -> Self {
        {
            let mut inner = unsafe { self.inner.as_ref() }.borrow_mut();
            inner.rc = inner.rc.saturating_add(1);
        }
        Self {
            inner: self.inner,
            _phantom: PhantomData {},
        }
    }
}
impl Drop for Ring {
    fn drop(&mut self) {
        {
            let mut inner = unsafe { self.inner.as_mut() }.borrow_mut();
            let rc = inner.rc;
            if rc > 1 {
                inner.rc = rc - 1;
                return;
            }
        }
        unsafe {
            let _drop = Box::from_raw(self.inner.as_ptr());
        }
    }
}
impl Ring {
    pub fn new(driver: Box<Driver>) -> Self {
        let boxed = Box::new(RefCell::new(RingInner::new(driver)));
        Self {
            inner: NonNull::new(Box::into_raw(boxed)).unwrap(), // SAFETY: unwrap is OK, the pointer is not null
            _phantom: PhantomData {},
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
