use std::{cell::RefCell, marker::PhantomData, ptr::NonNull, rc::Rc, sync::atomic::AtomicU32};

use crate::{
    driver_waker::DriverWaker, request_queue::RequestQueue, Driver, DriverIFace, PhantomUnsend,
    PhantomUnsync, ReadyList, Request,
};
use std::io::{Error, ErrorKind, Result};

use crate::details::TimerHeap;

pub(crate) struct RingInner {
    rc: u32,
    arc: AtomicU32, // TODO: prefer counting the wakers
    driver: Box<Driver>,
    concurrent: RequestQueue,
    ready: ReadyList,
    timeouts: TimerHeap,
    _unsync: PhantomUnsync,
    _unsend: PhantomUnsend,
}

pub struct Ring {
    inner: Box<RefCell<RingInner>>,
}
impl Drop for Ring {
    fn drop(&mut self) {
        let inner = self.inner.borrow();
        if inner.rc > 1 || inner.arc.load(std::sync::atomic::Ordering::Relaxed) != 0 {
            log::warn!("Need to cancel everything and wait"); // TODO:
        }
    }
}
impl Ring {
    pub fn new(driver: Box<Driver>) -> Result<Self> {
        Ok(Self {
            inner: Box::new(RefCell::new(RingInner::new(driver)?)),
        })
    }
}

#[repr(transparent)]
pub struct Completion {
    inner: Request,
}

impl Drop for Completion {
    fn drop(&mut self) {
        // let _ring = self.inner.owner.borrow_mut();

        // if let Some(&mut ring) = self.inner.owner {}
    }
}

impl Ring {
    pub fn submit(&self, sub: &mut Request) -> Result<&mut Completion> {
        // todo!(); TODO:
        Ok(unsafe { std::mem::transmute::<&mut Request, &mut &mut Completion>(sub) })
        // SAFETY: `transmute()` because both are same type
    }
}

impl RingInner {
    fn new(driver: Box<Driver>) -> Result<Self> {
        let timer_capacity = driver.config().submission_queue_depth as usize;
        let timer_capacity = if timer_capacity < 64 {
            64
        } else {
            timer_capacity
        };
        Ok(Self {
            rc: 1 as _,
            arc: AtomicU32::new(0u32),
            driver,
            concurrent: RequestQueue::new(),
            ready: ReadyList::new(),
            timeouts: TimerHeap::new(timer_capacity)?,
            _unsync: PhantomUnsync {},
            _unsend: PhantomUnsend {},
        })
    }
    fn cancel(_sub: &Completion) {}
    pub fn wait(&mut self) {
        let need_park: bool = false;
        if need_park {
            // let _scoped_parker =
            //     RequestQueueParkScope::new(&mut self.concurrent, &mut self.ready, need_park);
            todo!();
        }
    }
}
