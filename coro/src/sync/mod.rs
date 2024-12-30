use std::sync::Arc;

pub mod parking_lot;
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
