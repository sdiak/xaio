use std::io::Result;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::sys::ThreadId;
use crate::{io_req_fifo::IoReqFifo, io_req_lifo::IoReqLifo, IoReq};

const PARK_BIT: usize = 1;

pub type ParkerUnparkCb = unsafe extern "C" fn(thiz: NonNull<()>) -> ();

pub struct CompletionQueue(Arc<Inner>, crate::PhantomUnsend, crate::PhantomUnsync);

#[derive(Clone)]
pub struct Sender(Arc<Inner>);
impl Sender {
    pub fn send(&self, mut ready_list: IoReqLifo) {
        if !ready_list.is_empty() {
            self.0.push_bulk(ready_list.head, ready_list.tail);
            ready_list.head = std::ptr::null_mut();
            ready_list.tail = std::ptr::null_mut();
        }
    }
}

impl CompletionQueue {
    pub fn new(parker_unpark_cb: ParkerUnparkCb, parker: NonNull<()>) -> Result<Self> {
        Ok(Self(
            crate::catch_enomem(|| {
                Arc::new(Inner {
                    owner_thread_id: ThreadId::current(),
                    tail: AtomicUsize::new(0),
                    parker_unpark_cb,
                    parker,
                })
            })?,
            crate::PhantomUnsend {},
            crate::PhantomUnsync {},
        ))
    }

    pub fn new_sender(&self) -> Sender {
        Sender(self.0.clone())
    }

    pub fn park_begin(&self, ready_list: &mut IoReqFifo) -> usize {
        self.0.park_begin(ready_list)
    }

    pub fn park_end(&self, ready_list: &mut IoReqFifo) -> usize {
        self.0.park_begin(ready_list)
    }
}

struct Inner {
    owner_thread_id: ThreadId,
    tail: AtomicUsize,
    parker_unpark_cb: ParkerUnparkCb,
    parker: NonNull<()>,
}
impl Inner {
    fn push_bulk(&self, head: *mut IoReq, tail: *mut IoReq) -> bool {
        let mut old_tail = self.tail.load(Ordering::Acquire);
        loop {
            unsafe { (*tail).list_update_next((old_tail & !PARK_BIT) as _, Ordering::Relaxed) };
            match self.tail.compare_exchange_weak(
                old_tail,
                head as usize,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    if old_tail == PARK_BIT {
                        unsafe { (self.parker_unpark_cb)(self.parker) };
                        return true;
                    }
                    return false;
                }
                Err(t) => {
                    old_tail = t;
                }
            }
        }
    }

    fn park_begin(&self, ready_list: &mut IoReqFifo) -> usize {
        if self.owner_thread_id != ThreadId::current() {
            // Mostly for c-binding
            eprintln!("CompletionQueue::park_begin() can only be called from the owner thread");
            std::process::abort();
        }
        let old_tail = self.tail.swap(PARK_BIT, Ordering::Acquire);
        if old_tail == 0 {
            0
        } else {
            debug_assert!(
                old_tail != PARK_BIT,
                "The park-bit can not be set at this stage"
            );
            Self::reverse_list(old_tail as _, ready_list)
        }
    }

    fn park_end(&self, ready_list: &mut IoReqFifo) -> usize {
        if self.owner_thread_id != ThreadId::current() {
            // Mostly for c-binding
            eprintln!("CompletionQueue::park_end() can only be called from the owner thread");
            std::process::abort();
        }
        let old_tail = self.tail.swap(0, Ordering::Acquire);
        if old_tail == PARK_BIT {
            0
        } else {
            debug_assert!(
                old_tail != 0,
                "The park-bit is either set or the queue is not empty"
            );
            Self::reverse_list(old_tail as _, ready_list)
        }
    }

    fn reverse_list(src_tail: *mut IoReq, dst: &mut IoReqFifo) -> usize {
        let mut len = 0usize;
        let tail: *mut IoReq = src_tail;
        let mut head = src_tail;
        let mut prev = std::ptr::null_mut::<IoReq>();
        while !head.is_null() {
            len += 1;
            let next = unsafe { (*head).list_get_next(Ordering::Relaxed) };
            unsafe { (*head).list_update_next(prev, Ordering::Relaxed) };
            prev = head;
            head = next;
        }
        dst.push_back_all(&mut IoReqFifo { head: prev, tail });
        len
    }
}
