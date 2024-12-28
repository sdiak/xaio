use std::fmt::Debug;
mod dummy;
pub use dummy::*;

use enum_dispatch::enum_dispatch;

use crate::{collection::SList, Request};

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
