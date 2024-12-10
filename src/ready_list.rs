use std::sync::atomic::Ordering;

use crate::{request, Request};

#[derive(Debug)]
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

    pub(crate) unsafe fn pop_front(&mut self) -> *mut Request {
        let old_head = self.head;
        if !old_head.is_null() {
            self.head = (*old_head).list_pop_next(Ordering::Relaxed);
            if self.head.is_null() {
                self.tail = std::ptr::null_mut();
            }
            self.len -= 1;
        }
        old_head
    }

    pub(crate) unsafe fn push_back(&mut self, new_tail: *mut Request) {
        assert!(!new_tail.is_null() && (*new_tail).status != request::PENDING);
        (*new_tail).list_set_next(std::ptr::null_mut(), Ordering::Relaxed);
        if self.tail.is_null() {
            self.head = new_tail;
        } else {
            (*self.tail).list_update_next(new_tail, Ordering::Relaxed);
        }
        self.tail = new_tail;
        self.len += 1;
    }

    pub fn push_back_all(&mut self, other: &mut ReadyList) -> usize {
        if other.len() == 0 {
            return 0;
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple() {
        let mut a = Request::default();
        let mut b = Request::default();
        let mut c = Request::default();
        let mut d = Request::default();
        let mut ready0 = ReadyList::new();
        assert_eq!(unsafe { ready0.pop_front() }, std::ptr::null_mut());
        assert_eq!(ready0.len(), 0);

        unsafe { ready0.push_back(&mut a as *mut Request) };
        assert_eq!(ready0.len(), 1);

        assert_eq!(unsafe { ready0.pop_front() }, &mut a as *mut Request);
        assert_eq!(ready0.len(), 0);

        unsafe { ready0.push_back(&mut a as *mut Request) };
        assert_eq!(ready0.len(), 1);
        unsafe { ready0.push_back(&mut b as *mut Request) };
        assert_eq!(ready0.len(), 2);
        unsafe { ready0.push_back(&mut c as *mut Request) };
        assert_eq!(ready0.len(), 3);

        assert_eq!(unsafe { ready0.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready0.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready0.pop_front() }, &mut c as *mut Request);
        assert_eq!(ready0.len(), 0);

        let mut ready1 = ReadyList::new();
        unsafe { ready0.push_back(&mut a as *mut Request) };
        assert_eq!(ready0.len(), 1);
        unsafe { ready0.push_back(&mut b as *mut Request) };
        assert_eq!(ready0.len(), 2);
        unsafe { ready1.push_back(&mut c as *mut Request) };
        assert_eq!(ready1.len(), 1);

        assert_eq!(ready0.push_back_all(&mut ready1), 1);
        assert_eq!(ready0.len(), 3);
        assert_eq!(ready1.len(), 0);
        assert_eq!(unsafe { ready0.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready0.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready0.pop_front() }, &mut c as *mut Request);
        assert_eq!(ready0.len(), 0);
        // println!(" * * * ready1: {ready1:?}");

        unsafe { ready1.push_back(&mut a as *mut Request) };
        // println!(" * * * ready1: {ready1:?}");
        unsafe { ready1.push_back(&mut b as *mut Request) };
        // println!(" * * * ready1: {ready1:?}");
        unsafe { ready1.push_back(&mut c as *mut Request) };
        // println!(" * * * ready1: {ready1:?}");
        assert_eq!(ready1.len(), 3);
        assert_eq!(ready0.push_back_all(&mut ready1), 3);
        assert_eq!(ready0.len(), 3);
        assert_eq!(ready1.len(), 0);
        assert_eq!(unsafe { ready0.pop_front() }, &mut a as *mut Request);
        assert_eq!(unsafe { ready0.pop_front() }, &mut b as *mut Request);
        assert_eq!(unsafe { ready0.pop_front() }, &mut c as *mut Request);
        assert_eq!(ready0.len(), 0);

        unsafe { ready0.push_back(&mut a as *mut Request) };
        assert_eq!(ready0.len(), 1);
        assert_eq!(ready0.push_back_all(&mut ready1), 0);
        assert_eq!(unsafe { ready0.pop_front() }, &mut a as *mut Request);
        assert_eq!(ready0.len(), 0);
    }
}
