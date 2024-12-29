use std::sync::Arc;

use crate::{collection::SList, sync::Queue, Request};

fn create_singleton() -> DummyDriver {
    let (tx, rx) = std::sync::mpsc::sync_channel::<DummyDriver>(1);
    let _ = std::thread::spawn(move || {
        let thiz = DummyDriver::__create();
        tx.send(thiz.clone()).expect("Unrecoverable error");
        drop(tx);
        thiz.__thread_run();
    });
    rx.recv().expect("Unrecoverable error")
}
static INSTANCE: std::sync::LazyLock<DummyDriver> = std::sync::LazyLock::new(create_singleton);

use super::DriverTrait;

#[derive(Debug, Clone)]
pub struct DummyDriver(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    thread: std::thread::Thread,
    queue: Queue<Request>,
}
impl Inner {
    fn new() -> Self {
        Self {
            thread: std::thread::current(),
            queue: Queue::new(),
        }
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
        let mut batcher = super::DriverCompletionPortBatcher::new();
        let mut requests = SList::<Request>::new();
        loop {
            self.0.queue.park(
                |todo: &mut SList<Request>| {
                    if todo.is_empty() {
                        std::thread::park();
                    }
                    0
                },
                &mut requests,
            );
            while let Some(req) = requests.pop_front() {
                batcher.push(req, -libc::ENOSYS);
            }
            batcher.finish();
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
