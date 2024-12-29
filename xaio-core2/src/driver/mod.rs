use std::fmt::Debug;
mod dummy;
pub use dummy::*;

use enum_dispatch::enum_dispatch;

use crate::{collection::SList, CompletionPort, Ptr, Request};

#[enum_dispatch]
pub trait DriverTrait: Clone + Debug {
    // type Sender: Sender;

    // fn sender(&self) -> Sender;
    fn submit(&self, requests: &mut SList<Request>);
}

#[enum_dispatch(DriverTrait)]
#[derive(Debug, Clone)]
pub enum Driver {
    DummyDriver,
}

struct DriverCompletionPortBatch {
    batch: SList<Request>,
    len: usize,
}
struct DriverCompletionPortBatcher(rustc_hash::FxHashMap<usize, DriverCompletionPortBatch>);
impl DriverCompletionPortBatcher {
    pub(crate) fn new() -> Self {
        Self(
            rustc_hash::FxHashMap::<usize, DriverCompletionPortBatch>::with_hasher(
                rustc_hash::FxBuildHasher,
            ),
        )
    }
    pub(crate) fn push(&mut self, mut req: Ptr<Request>, status: i32) {
        req.set_status_from_driver(status);
        // Safety: CompletionPort is heap-allocated, it's address is unique and pinned.
        // It's kept alive because live request keeps a reference to it
        let cp_addr = req.completion_port() as *const CompletionPort as usize;
        if let Some(cp) = self.0.get_mut(&cp_addr) {
            cp.batch.push_back(req);
            cp.len += 1;
        } else if self.0.try_reserve(1).is_ok() {
            self.0.insert(
                cp_addr,
                DriverCompletionPortBatch {
                    batch: SList::from_node(req),
                    len: 1,
                },
            );
        } else {
            // Out of memory, just push it
            let cp = unsafe { &*(cp_addr as *const CompletionPort) };
            cp.done(&mut SList::from_node(req), 1);
        }
    }
    pub(crate) fn finish(&mut self) {
        for (cp_addr, mut batch) in self.0.drain() {
            let cp = unsafe { &*(cp_addr as *const CompletionPort) };
            cp.done(&mut batch.batch, batch.len);
        }
    }
}
