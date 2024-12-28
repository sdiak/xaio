use std::fmt::Debug;
mod dummy;
pub use dummy::*;

use enum_dispatch::enum_dispatch;

pub trait Sender: Clone + Debug {
    fn submit(&self, req: crate::Ptr<crate::Request>);
    fn flush(&self) -> usize;
}

#[enum_dispatch]
pub trait DriverTrait: Clone + Debug {
    type Sender: Sender;

    fn sender(&self) -> Self::Sender;
}
