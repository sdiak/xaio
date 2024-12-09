use std::sync::atomic::AtomicUsize;
use std::{ptr, sync::atomic::Ordering};

use crate::{ReadyList, Request};

const PARK_BIT: usize = 1usize;

pub(crate) struct RequestQueue {
    tail: AtomicUsize,
}

pub(crate) struct RequestQueueParkScope<'scope> {
    queue: &'scope mut RequestQueue,
    ready: &'scope mut ReadyList,
    parked: bool,
}
impl<'scope> RequestQueueParkScope<'scope> {
    pub(crate) fn new(
        queue: &'scope mut RequestQueue,
        ready: &'scope mut ReadyList,
        need_park: bool,
    ) -> Self {
        let mut parked = false;
        let mut need_park = need_park;
        while need_park && !parked {
            match queue.tail.compare_exchange_weak(
                0,
                PARK_BIT,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    parked = true;
                }
                Err(current) => {
                    need_park = current == 0usize;
                }
            }
        }
        Self {
            queue,
            ready,
            parked,
        }
    }
}
impl<'scope> Drop for RequestQueueParkScope<'scope> {
    fn drop(&mut self) {
        if self.parked {
            let mut tail: *mut Request =
                (self.queue.tail.swap(0, Ordering::Acquire) & !PARK_BIT) as _;
            if !tail.is_null() {
                self.ready.transfert_from(&mut reverse_list(tail));
            }
        }
    }
}

fn reverse_list(old_head: *mut Request) -> ReadyList {
    let mut len = 0usize;
    let tail = old_head;
    let mut head = old_head;
    let mut prev = std::ptr::null_mut::<Request>();
    while !head.is_null() {
        len += 1;
        let next = unsafe { (*head).list_get_next(Ordering::Relaxed) };
        unsafe { (*head).list_update_next(prev, Ordering::Relaxed) };
        prev = head;
        head = next;
    }
    ReadyList { head, tail, len }
}

impl RequestQueue {
    pub(crate) fn new() -> RequestQueue {
        RequestQueue {
            tail: AtomicUsize::new(0usize),
        }
    }

    /// Adds a new completed request to the given concurrent queue.
    ///
    /// # Arguments
    ///   - `req` The completed request.
    pub(crate) unsafe fn push(&mut self, req: *mut Request) {
        debug_assert!(!req.is_null());
        // Ensures in a single list at a given time
        (*req).list_set_next(std::ptr::null_mut(), Ordering::Relaxed);
        let mut old_tail = self.tail.load(Ordering::Acquire);
        loop {
            (*req).list_update_next((old_tail & !PARK_BIT) as _, Ordering::Relaxed);
            match self.tail.compare_exchange_weak(
                old_tail,
                req as usize,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    if old_tail == 0 || (old_tail & PARK_BIT) != 0 {
                        // TODO: notify
                    }
                    return;
                }
                Err(t) => {
                    old_tail = t;
                }
            }
        }
    }
}
