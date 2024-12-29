use std::sync::Arc;

use crate::Request;
mod smpsc;
pub(crate) use smpsc::Queue;

pub(crate) trait Park: std::fmt::Debug + Clone + std::panic::UnwindSafe {
    fn park(&self, timeout_ms: i32);
    fn unpark(&self);
}

#[derive(Debug, Clone)]
pub struct ThreadPark {
    thread: std::thread::Thread,
}
impl Park for ThreadPark {
    fn park(&self, timeout_ms: i32) {
        if timeout_ms < 0 {
            std::thread::park();
        } else {
            std::thread::park_timeout(std::time::Duration::from_millis(timeout_ms as _));
        }
    }
    fn unpark(&self) {
        self.thread.unpark();
    }
}

pub(crate) struct ConcurrentRequestQueue<P: Park, U: std::panic::UnwindSafe> {
    pub(crate) user: U,
    parker: P,
    queue: Queue<Request>,
}
impl<P: Park, U: std::panic::UnwindSafe> ConcurrentRequestQueue<P, U> {
    pub(crate) fn new(user: U, parker: P) -> Arc<Self> {
        Arc::new(Self {
            user,
            parker,
            queue: Queue::new(),
        })
    }
    #[inline(always)]
    pub(crate) fn try_new(user: U, parker: P) -> std::io::Result<Arc<Self>> {
        crate::catch_enomem(move || ConcurrentRequestQueue::new(user, parker))
    }

    #[inline]
    pub(crate) fn submit(&self, requests: &mut crate::collection::SList<Request>) {
        if self.queue.append(requests) {
            self.parker.unpark();
        }
    }

    #[inline(always)]
    pub(crate) fn wait_fn<F: FnOnce(&mut crate::collection::SList<Request>) -> usize>(
        &self,
        f: F,
        ready_sink: &mut crate::collection::SList<Request>,
    ) -> usize {
        self.queue.park(f, ready_sink)
    }

    pub(crate) fn wait(
        &self,
        ready_sink: &mut crate::collection::SList<Request>,
        timeout_ms: i32,
    ) -> usize {
        self.wait_fn(
            |available| {
                if available.is_empty() {
                    self.parker.park(timeout_ms);
                }
                0
            },
            ready_sink,
        )
    }
}
