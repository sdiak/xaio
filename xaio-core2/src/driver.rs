use std::fmt::Debug;

pub trait Sender: Clone + Debug {
    fn submit(&self, req: crate::Ptr<crate::Request>);
    fn flush(&self) -> usize;
}
pub trait Driver: Clone + Debug {
    type Sender: Sender;

    fn sender(&self) -> Self::Sender;
}
