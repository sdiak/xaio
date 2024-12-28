use std::{
    mem::offset_of,
    sync::atomic::{AtomicI32, Ordering},
};

use crate::{collection::SListNode, Ptr};

pub const PENDING: i32 = i32::MIN;

#[derive(Debug)]
pub struct Request {
    // port: &'static CompletionPort<>
    link: crate::collection::SLink,
    status: AtomicI32,
}

impl SListNode for Request {
    const OFFSET_OF_LINK: usize = offset_of!(Request, link);
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
        ) {
            // TODO: submit
        }
    }
}
// pub type Callback = fn(Ptr<Request>) -> Option<Ptr<Request>>;
