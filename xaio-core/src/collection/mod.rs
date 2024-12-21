mod snode;
use std::marker::PhantomData;

pub use snode::*;
mod slist;
pub use slist::*;

pub use std::ops::Deref;

pub struct NodeRef<'a, Collection, T> {
    val: &'a T,
    borrow: PhantomData<&'a Collection>,
}
impl<'a, Collection, T> NodeRef<'a, Collection, T> {
    pub(crate) fn new(_col: &'a Collection, val: &'a T) -> Self {
        Self {
            val,
            borrow: PhantomData {},
        }
    }
}
impl<'a, Collection, T> Deref for NodeRef<'a, Collection, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.val
    }
}
