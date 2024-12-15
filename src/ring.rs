use crate::{
    request, sys, thread_pool, OpCode, Status, FLAG_INITIALIZED, FLAG_RING_OWNED, PENDING,
};
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::io::Error;
use std::mem::{zeroed, ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::panic::UnwindSafe;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use std::{cell::RefCell, sync::atomic::AtomicU32, time::Instant};

use crate::{
    request_queue::RequestQueue, Driver, DriverIFace, PhantomUnsend, PhantomUnsync, ReadyList,
    Request, RequestCallback,
};
use std::io::{ErrorKind, Result};

use crate::details::TimerHeap;

static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(crate) struct RingInner {
    rc: u32,
    arc: AtomicU32, // TODO: prefer counting the wakers
    epoch: Instant,
    now_ms: u64,
    driver: Box<Driver>,
    concurrent: RequestQueue,
    ready: ReadyList,
    timeouts: TimerHeap,
    interrupts: FxHashMap<sys::Event, RingInterrupt>,
    _unsync: PhantomUnsync,
    _unsend: PhantomUnsend,
}

#[derive(Clone)]
pub struct Ring {
    inner: Rc<RefCell<RingInner>>,
}
impl Drop for Ring {
    fn drop(&mut self) {
        let inner = self.inner.borrow();
        if inner.rc > 1 || inner.arc.load(std::sync::atomic::Ordering::Relaxed) != 0 {
            log::warn!("Need to cancel everything and wait"); // TODO:
        }
    }
}

struct DropReqOnPanic<'a> {
    ring: &'a Ring,
    req: NonNull<Request>,
}
impl<'a> Drop for DropReqOnPanic<'a> {
    fn drop(&mut self) {
        self.ring.__drop_request(self.req);
    }
}
impl<'a> DropReqOnPanic<'a> {
    fn as_mut(&mut self) -> &mut Request {
        unsafe { self.req.as_mut() }
    }
}

impl Ring {
    pub fn new(driver: Box<Driver>) -> Result<Self> {
        Ok(Self {
            inner: Rc::new(RefCell::new(RingInner::new(driver)?)),
        })
    }

    pub fn wait_ms(&self, timeout_ms: i32) -> Status {
        let mut ready: ReadyList = ReadyList::new();
        let mut thiz = self.inner.borrow_mut();
        let timeout_ms = unsafe { thiz.concurrent.park_begin(timeout_ms) };
        match thiz.driver.wait(&mut ready, timeout_ms) {
            Ok(_) => {}
            Err(err) => return Status::from(err),
        };
        unsafe { thiz.concurrent.park_end(&mut ready) };
        let mut it = ready.head;
        let mut n_events = ready.len() as i32;
        while !it.is_null() {
            let req = it;
            it = unsafe { (*it).list_get_next(Ordering::Relaxed) };
            match unsafe { (*req).callback } {
                Some(cb) => {
                    cb(unsafe { &mut *req });
                }
                None => {}
            }
        }
        // All the events are handled
        std::mem::forget(ready);
        Status::new(n_events)
    }
    pub fn submit_io_work<W>(
        &self,
        work: W,
        callback: Option<RequestCallback>,
        memory: Option<NonNull<Request>>,
    ) -> Result<()>
    where
        W: FnOnce() -> i32 + Send + UnwindSafe + 'static,
    {
        let mut memory = self.__get_or_allocate_req(memory, OpCode::IO_WORK, callback)?;
        unsafe {
            std::ptr::write(
                &mut *memory.as_mut().op.rust_work as *mut request::RustWork,
                request::RustWork {
                    work: Some(Box::new(work)),
                    panic_cause: None,
                },
            );
            memory.as_mut().flags_and_op_code |= request::FLAG_NEED_DROP;
        }
        self.__submit(memory);
        Ok(())
    }

    fn __submit(&self, mut prepared: DropReqOnPanic) {
        println!("SUBMIT");
        let ring = self.inner.borrow_mut();
        match prepared.as_mut().opcode_raw() {
            _ => thread_pool::submit_io_work(&ring.concurrent, prepared.req),
        };
        // drop on panic no longer needed
        std::mem::forget(prepared);
    }
    fn __allocate_request(&self) -> Result<NonNull<Request>> {
        crate::catch_enomem(|| unsafe {
            NonNull::new_unchecked(
                Box::<Request>::leak(Box::<Request>::new(Request::default())) as *mut Request,
            )
        })
    }
    fn __drop_request(&self, mut req: NonNull<Request>) {
        let req = unsafe { req.as_mut() };
        if (req.flags_and_op_code & request::FLAG_NEED_DROP) != 0 {
            match req.opcode_raw() {
                // TODO: operation destructors
                _ => {}
            }
        }
        if (req.flags_and_op_code & request::FLAG_RING_OWNED) != 0 {
            let _ = unsafe { Box::<request::Request>::from_raw(req as *mut Request) };
        }
    }
    fn __get_or_allocate_req(
        &self,
        memory: Option<NonNull<Request>>,
        opcode: request::OpCode,
        callback: Option<RequestCallback>,
    ) -> Result<DropReqOnPanic> {
        let mut flags = FLAG_INITIALIZED | opcode as u32;
        let mut memory = match memory {
            Some(mut memory) => {
                if !unsafe { memory.as_ref().is_new() } {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
                memory
            }
            None => {
                flags |= FLAG_RING_OWNED;
                self.__allocate_request()?
            }
        };
        {
            let req = unsafe { memory.as_mut() };
            req.flags_and_op_code = flags;
            req.status.store(PENDING, Ordering::Relaxed);
            req.callback = callback;
        }
        Ok(DropReqOnPanic {
            ring: self,
            req: memory,
        })
    }

    pub fn add_interrupt(&mut self, interrupt: RingInterrupt) -> Result<sys::Event> {
        let mut thiz = self.inner.borrow_mut();
        if thiz.interrupts.try_reserve(1).is_ok() {
            let ev = sys::Event::new()?;
            thiz.interrupts.insert(ev.clone(), interrupt);
            // TODO: driver
            Ok(ev)
        } else {
            Err(Error::from(ErrorKind::OutOfMemory))
        }
    }
    pub fn remove_interrupt(&mut self, event: &sys::Event) -> bool {
        let mut thiz = self.inner.borrow_mut();
        if thiz.interrupts.remove(event).is_some() {
            //TODO: driver
            true
        } else {
            false
        }
    }
}

pub enum RingInterrupt {
    Rust(Box<dyn FnMut()>),
    C(extern "C" fn(*mut libc::c_void), *mut libc::c_void),
}

impl std::fmt::Debug for RingInterrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust(_) => f.write_str("RingInterrupt::Rust(Box<dyn FnMut()>)"),
            Self::C(_, _) => f.write_str(
                "RingInterrupt::C(extern \"C\" fn(*mut libc::c_void), *mut libc::c_void)",
            ),
        }
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
            epoch: *EPOCH,
            now_ms: EPOCH.elapsed().as_millis() as u64, // SAFETY: program is not expected to run more than ~ 2.5e+13 years
            driver,
            concurrent: RequestQueue::new()?,
            ready: ReadyList::new(),
            timeouts: TimerHeap::new(timer_capacity)?,
            interrupts: FxHashMap::<sys::Event, RingInterrupt>::with_capacity_and_hasher(
                32,
                FxBuildHasher,
            ),
            _unsync: PhantomUnsync {},
            _unsend: PhantomUnsend {},
        })
    }
    pub fn update_now(&mut self) -> u64 {
        self.now_ms = self.epoch.elapsed().as_millis() as u64; // SAFETY: program is not expected to run more than ~ 2.5e+13 years
        self.now_ms
    }
    pub fn now(&self) -> u64 {
        self.now_ms
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
