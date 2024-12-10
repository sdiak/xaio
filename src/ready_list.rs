use std::sync::atomic::Ordering;

use crate::{request, Request};

pub struct ReadyList {
    pub(crate) head: *mut Request,
    pub(crate) tail: *mut Request,
    pub(crate) len: usize,
}
impl Drop for ReadyList {
    fn drop(&mut self) {
        debug_assert!(self.len() == 0);
    }
}

impl ReadyList {
    pub fn new() -> Self {
        Self {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub(crate) unsafe fn push_back(&mut self, new_tail: *mut Request) {
        assert!(!new_tail.is_null() && (*new_tail).status != request::PENDING);
        (*new_tail).list_set_next(self.tail, Ordering::Relaxed);
        if self.tail.is_null() {
            self.head = new_tail;
        }
        self.tail = new_tail;
        self.len += 1;
    }

    pub fn push_back_all(&mut self, other: &mut ReadyList) -> usize {
        if self.tail.is_null() {
            self.head = other.head;
        } else {
            unsafe { (*(self.tail)).list_update_next(other.head, Ordering::Relaxed) };
        }
        self.tail = other.tail;
        let transfered = other.len();
        other.len = 0;
        other.head = std::ptr::null_mut();
        other.tail = std::ptr::null_mut();
        self.len += transfered;
        transfered
    }
}
