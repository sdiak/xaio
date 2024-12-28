use std::{
    mem::offset_of,
    sync::atomic::{AtomicI32, Ordering},
};

use crate::{collection::SListNode, CompletionPort, Ptr};

pub const PENDING: i32 = i32::MIN;
pub const UNKNWON: i32 = PENDING + 1;

#[derive(Debug)]
pub struct Request {
    completion_port: usize,
    link: crate::collection::SLink,
    status: AtomicI32,
}
unsafe impl Send for Request {}
unsafe impl Send for Ptr<Request> {}

impl Request {
    pub(crate) fn completion_port(&self) -> &crate::CompletionPort {
        unsafe { &*(self.completion_port as *const crate::CompletionPort) }
    }

    #[inline(always)]
    pub(crate) fn cancel(ptr: &mut Ptr<Request>) {
        if let Ok(_) = ptr.status.compare_exchange(
            PENDING,
            -libc::ECANCELED,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            ptr.completion_port().cancel_hint(ptr);
        }
    }
    pub(crate) fn done(ptr: Ptr<Request>, status: i32) {
        // Pending becomes unknown
        let status = status + ((status == PENDING) as i32);
        // We don't care if a cancelled task succeed, so we do not use CAS ; just a plain store
        ptr.status.store(status, Ordering::Relaxed);
        todo!("Queue to completion port");
    }
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
    #[inline(always)]
    fn drop(&mut self) {
        Request::cancel(&mut self.0);
    }
}
// pub type Callback = fn(Ptr<Request>) -> Option<Ptr<Request>>;
