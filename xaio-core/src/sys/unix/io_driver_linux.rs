use crate::collection::smpsc;
use crate::{IoReq, Unparker};
use std::io::Result;
use std::mem::ManuallyDrop;
use std::sync::Arc;

use super::eventfd::EventFd;

pub type Sender = smpsc::BufferedSender<IoReq>;

pub struct IoDriver {
    inner: Arc<Inner>,
}

struct Inner {
    requests: smpsc::Receiver<IoReq>,
    waker: EventFd,
}

struct EventFdUnpark(ManuallyDrop<EventFd>);
impl EventFdUnpark {
    fn new(evfd: &EventFd) -> Self {
        Self(ManuallyDrop::new(EventFd {
            handle: evfd.handle,
        }))
    }
}
impl crate::Unpark for EventFdUnpark {
    fn unpark(&self) {
        self.0.write(1).expect("Writing to eventfd should not fail");
    }
}
impl Clone for EventFdUnpark {
    fn clone(&self) -> Self {
        Self(ManuallyDrop::new(EventFd {
            handle: self.0.handle,
        }))
    }
}
impl Inner {
    fn try_new() -> Result<Self> {
        let waker = EventFd::new(0, false)?;
        let target = Unparker::new(EventFdUnpark::new(&waker));
        Ok(Self {
            requests: smpsc::Receiver::new(target),
            waker,
        })
    }

    fn wake(&self) {
        self.waker
            .write(1)
            .expect("Writing to eventfd should not fail");
    }
}
