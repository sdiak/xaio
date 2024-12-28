use std::sync::atomic::{AtomicI32, Ordering};

use crate::Ptr;

pub const PENDING: i32 = i32::MIN;

pub struct Request {
    // port: &'static CompletionPort<>
    status: AtomicI32,
}

#[repr(transparent)]
pub struct Handle(Ptr<Request>);

impl Handle {
    pub(crate) fn new(req: &mut Ptr<Request>) -> Self {
        Self(unsafe { Ptr::from_raw_unchecked(req.as_ptr()) })
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        if let Ok(_) = self.0.status.compare_exchange(
            PENDING,
            -libc::ECANCELED,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {}
    }
}
// pub type Callback = fn(Ptr<Request>) -> Option<Ptr<Request>>;
