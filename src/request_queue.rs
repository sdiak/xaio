use std::sync::atomic::AtomicUsize;
use std::{ptr, sync::atomic::Ordering};

use crate::{request, ReadyList, Request, PENDING, UNKNOWN};

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
        let tail: *mut Request = (self.queue.tail.swap(0, Ordering::Acquire) & !PARK_BIT) as _;
        if !tail.is_null() {
            self.ready.push_back_all(&mut reverse_list(tail));
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
        unsafe {
            // SAFETY: called from owner thread
            (*head).status = (*head).concurrent_status.load(Ordering::Relaxed);
        }
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
        assert!(
            !req.is_null() && (*req).concurrent_status.load(Ordering::Relaxed) != request::PENDING
        );
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple() {
        let mut a = Request::default();
        let mut b = Request::default();
        let mut c = Request::default();
        let mut d = Request::default();
        let mut rq = RequestQueue::new();
        let mut ready = ReadyList::new();
        unsafe {
            rq.push(&mut a as *mut Request);
            rq.push(&mut b as *mut Request);
            rq.push(&mut c as *mut Request);
        }
        {
            let scope = RequestQueueParkScope::new(&mut rq, &mut ready, false);
            // unsafe {
            //     rq.push(&mut d as *mut Request);
            // }
        }
        assert_eq!(unsafe { ready.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut c as *mut Request);
        // assert_eq!(unsafe { l.pop_front() }, &mut a as *mut Request);
        std::mem::forget(ready);
    }
}
