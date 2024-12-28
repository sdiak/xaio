use std::sync::Arc;

use crate::{
    collection::{smpsc, SList},
    Request,
};

fn create_instance() -> DummyDriver {
    let (tx, rx) = std::sync::mpsc::sync_channel::<DummyDriver>(1);
    let _ = std::thread::spawn(move || {
        let mut thiz = DummyDriver::__create();
        tx.send(thiz.clone()).expect("Unrecoverable error");
        drop(tx);
        thiz.__thread_run();
    });
    rx.recv().expect("Unrecoverable error")
}
static INSTANCE: std::sync::LazyLock<DummyDriver> = std::sync::LazyLock::new(create_instance);

use super::DriverTrait;

#[derive(Debug, Clone)]
pub struct DummyDriver(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    thread: std::thread::Thread,
    queue: smpsc::Queue<Request, crate::ThreadUnpark>,
}
impl Inner {
    fn new() -> Self {
        todo!()
        // Self {
        //     thread: std::thread::current(),
        //     queue: smpsc::Queue::new(unpark)
        // }
    }
}

impl DummyDriver {
    pub fn new() -> Self {
        Self(INSTANCE.0.clone())
    }
    fn __create() -> Self {
        Self(Arc::new(Inner::new()))
    }
    fn __thread_run(self) {
        let mut requests = SList::<Request>::new();
        loop {
            self.0.queue.park(
                || {
                    //TODO:
                },
                &requests,
            );
        }
    }
}

impl DriverTrait for DummyDriver {
    fn submit(&self, requests: &mut SList<Request>) {
        if self.0.queue.append(requests) {
            self.0.thread.unpark();
        }
    }
}
