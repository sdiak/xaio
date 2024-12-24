use crate::io_driver::{IoDriver, TmpIoDriverSender};
use crate::sys::io_driver;
use crate::Unpark;
use crate::{collection::smpsc2::Queue, collection::SList, IoReq};
use std::io::Result;
use std::marker::PhantomData;
use std::sync::Arc;

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        type CellType<T> = std::cell::RefCell<T>;
    } else {
        type CellType<T> = std::cell::UnsafeCell<T>;
    }
}

pub struct CompletionPort(Arc<CellType<Inner>>);

impl CompletionPort {
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            #[inline(always)]
            fn inner_mut(&self) -> std::cell::RefMut<'_, Inner> {
                self.0.borrow_mut()
            }
        } else {
            #[inline(always)]
            fn inner_mut(&self) -> &mut Inner {
                unsafe { &mut *self.0.get() }
            }
        }
    }

    pub fn submit(&self, prepared_request: Box<IoReq>) {
        self.inner_mut().submit(prepared_request)
    }
    pub fn flush_submissions(&self) -> usize {
        self.inner_mut().flush_submissions()
    }
}

// TODO: pub to allow sending a message object to another port
pub struct CompletedIoReqSender(Arc<CellType<Inner>>);

impl CompletedIoReqSender {
    fn new(inner: Arc<CellType<Inner>>) -> Self {
        Self(inner)
    }
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            #[inline(always)]
            fn inner_mut(&self) -> std::cell::RefMut<'_, Inner> {
                self.0.borrow_mut()
            }
        } else {
            #[inline(always)]
            fn inner_mut(&self) -> &mut Inner {
                unsafe { &mut *self.0.get() }
            }
        }
    }
    pub(crate) fn _send_completed(&self, completed: &mut SList<IoReq>) {
        self.inner_mut()._send_completed(completed)
    }
}

struct Inner {
    owner_id: crate::sys::ThreadId,
    requests_in_flight: u32,
    submit_batch_len: u32,
    submit_batch: SList<IoReq>,
    // pending_submissions: usize,
    completed_len: u32,
    completed: SList<IoReq>,
    completed_queue: Queue<IoReq, InnerUnpark>,

    driver: TmpIoDriverSender, // TODO:

    owner: std::thread::Thread,
}
struct InnerUnpark(std::thread::Thread);
impl Unpark for InnerUnpark {
    fn unpark(&self) {
        self.0.unpark();
    }
}
impl Inner {
    fn new() -> Self {
        Self {
            owner: std::thread::current(),
            owner_id: crate::sys::ThreadId::current(),
            requests_in_flight: 0,
            submit_batch_len: 0,
            submit_batch: SList::new(),
            completed_len: 0,
            completed: SList::new(),
            driver: TmpIoDriverSender {},
            completed_queue: Queue::new(InnerUnpark(std::thread::current())),
        }
    }
    fn submit(&mut self, prepared_request: Box<IoReq>) {
        self.requests_in_flight += 1;
        self.submit_batch_len += 1;
        self.submit_batch.push_back(prepared_request);
    }
    fn flush_submissions(&mut self) -> usize {
        let len = self.submit_batch_len as usize;
        self.driver.submit(&mut self.submit_batch);
        self.submit_batch_len = 0;
        len
    }
    fn _send_completed(&mut self, completed: &mut SList<IoReq>) {
        // debug_assert!(completed.status() != IoReq::STATUS_PENDING);
        if crate::sys::ThreadId::current() == self.completed_queue.owner_thread_id() {
            // Fast path
            self.completed_len += 1;
            self.completed.append(completed);
        } else {
            // Slow path
            self.completed_queue.append(completed);
        }
    }
}
