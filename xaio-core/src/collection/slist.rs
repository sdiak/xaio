use std::{marker::PhantomData, pin::Pin};

use super::{SListLink, SListNode};

pub struct SList<T: SListNode> {
    pub(crate) head: *mut SListLink,
    pub(crate) tail: *mut SListLink,
    _phantom: PhantomData<T>,
}

impl<T: SListNode> SList<T> {
    pub fn new() -> SList<T> {
        Self {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
            _phantom: PhantomData::<T> {},
        }
    }

    pub fn push_back(&mut self, node: Pin<&mut T>) {}
}
