use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::{request, ReadyList, Request};

const PARK_BIT: usize = 1usize;

pub(crate) struct RequestQueue {
    tail: AtomicUsize,
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
    pub(crate) unsafe fn push(&self, req: *mut Request) {
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
                    if (old_tail & PARK_BIT) != 0 { // old_tail == 0 || (old_tail & PARK_BIT) != 0
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

    #[must_use]
    /// Prepare park, `RequestQueue::push` will wake the calling driver when data is available
    ///
    /// # Arguments
    ///   - `timeout_ms` The timeout in milliseconds, that the subsequent wait will use
    ///
    /// # Returns
    ///   `timeout_ms` or `0` when some request are already available.
    ///   The caller should use it like so : `timeout_ms = request_queue.park_begin(timeout_ms);`
    pub(crate) unsafe fn park_begin(&self, timeout_ms: i32) -> i32 {
        if timeout_ms > 0 {
            loop {
                match self.tail.compare_exchange_weak(
                    0,
                    PARK_BIT,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        return timeout_ms;
                    }
                    Err(current) => {
                        if current != 0usize {
                            // At least one ready requests is available ; do not wait
                            return 0;
                        }
                        // LCOV_EXCL_LINE : spurious failure, I do not know how to produce them in tests
                    }
                }
            }
        }
        0
    }

    /// Ends park
    ///
    /// # Arguments
    ///   - `ready_list` Receives concurrent ready requests
    ///
    /// # Returns
    ///   the number of concurrent ready requests that was moved to `ready_list`
    pub(crate) unsafe fn park_end(&self, ready_list: &mut ReadyList) -> i32 {
        let mut count = 0i32;
        if self.tail.load(Ordering::Relaxed) != 0 {
            let tail: *mut Request = (self.tail.swap(0, Ordering::Acquire) & !PARK_BIT) as _;
            if !tail.is_null() {
                let len_before = ready_list.len();
                ready_list.push_back_all(&mut RequestQueue::reverse_list(tail));
                count = (ready_list.len() - len_before) as i32;
            }
        }
        count
    }

    fn reverse_list(old_head: *mut Request) -> ReadyList {
        let mut len = 0usize;
        let tail: *mut Request = old_head;
        let mut head = old_head;
        let mut prev = std::ptr::null_mut::<Request>();
        while !head.is_null() {
            len += 1;
            unsafe {
                // SAFETY: called from request owner thread
                (*head).status = (*head).concurrent_status.load(Ordering::Relaxed);
            }
            let next = unsafe { (*head).list_get_next(Ordering::Relaxed) };
            unsafe { (*head).list_update_next(prev, Ordering::Relaxed) };
            prev = head;
            head = next;
        }
        ReadyList {
            head: prev,
            tail,
            len,
        }
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, thread};

    use super::*;

    unsafe impl Send for Request {}

    #[test]
    fn test_not_parked() {
        let mut a = Request::default();
        let mut b = Request::default();
        let mut c = Request::default();
        let mut rq = RequestQueue::new();
        let mut ready = ReadyList::new();
        unsafe {
            rq.push(&mut a as *mut Request);
            rq.push(&mut b as *mut Request);
            rq.push(&mut c as *mut Request);
        }
        {
            assert_eq!(unsafe { rq.park_begin(i32::MAX) }, 0);

            unsafe { rq.park_end(&mut ready) };
        }
        assert_eq!(unsafe { ready.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut c as *mut Request);

        {
            assert_eq!(unsafe { rq.park_begin(0) }, 0);

            unsafe { rq.park_end(&mut ready) };
        }
        assert_eq!(ready.len(), 0);
    }
    #[test]
    fn test_parked() {
        let mut a = Request::default();
        let mut b = Request::default();
        let mut c = Request::default();
        let mut d = Request::default();
        let mut rq = RequestQueue::new();
        let mut ready = ReadyList::new();
        {
            assert_eq!(unsafe { rq.park_begin(i32::MAX) }, i32::MAX);
            unsafe {
                rq.push(&mut a as *mut Request);
                rq.push(&mut b as *mut Request);
                rq.push(&mut c as *mut Request);
            }
            unsafe { rq.park_end(&mut ready) };
        }
        unsafe {
            ready.push_back(&mut d as *mut Request);
        }
        assert_eq!(unsafe { ready.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut c as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut d as *mut Request);

        {
            assert_eq!(unsafe { rq.park_begin(0) }, 0);
            unsafe {
                rq.push(&mut a as *mut Request);
                rq.push(&mut b as *mut Request);
                rq.push(&mut c as *mut Request);
            }
            unsafe { rq.park_end(&mut ready) };
        }
        unsafe {
            ready.push_back(&mut d as *mut Request);
        }
        assert_eq!(unsafe { ready.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut c as *mut Request);
        assert_eq!(unsafe { ready.pop_front() }, &mut d as *mut Request);
    }

    #[test]
    fn test_concurrent() {
        let t0_data: Vec<Request> = (0..1024).map(|_| Request::default()).collect();
        let t1_data: Vec<Request> = (0..1024).map(|_| Request::default()).collect();
        let expected = t0_data.len() + t1_data.len();
        let mut ready = ReadyList::new();
        let rq = Arc::new(RequestQueue::new());
        let rq0 = rq.clone();
        let rq1 = rq.clone();
        let t0 = thread::spawn(move || {
            for r in t0_data.iter() {
                unsafe {
                    rq0.push(r as *const Request as _);
                }
            }
            t0_data
        });
        let t1 = thread::spawn(move || {
            for r in t1_data.iter() {
                unsafe {
                    rq1.push(r as *const Request as _);
                }
            }
            t1_data
        });

        while ready.len() < expected {
            let timeout = unsafe { rq.park_begin(i32::MAX) };
            unsafe { rq.park_end(&mut ready) };
        }

        let t0_data = t0.join();
        assert!(t0_data.is_ok());
        let t1_data = t1.join();
        assert!(t1_data.is_ok());

        unsafe { rq.park_end(&mut ready) };
        assert_eq!(ready.len(), expected);
        for _ in 0..expected {
            assert_ne!(unsafe { ready.pop_front() }, std::ptr::null_mut() as _);
        }
        assert_eq!(ready.len(), 0);
    }
}
