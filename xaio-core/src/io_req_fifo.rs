use std::{ptr::NonNull, sync::atomic::Ordering};

use crate::IoReq;

#[derive(Debug)]
pub struct IoReqFifo {
    pub(crate) head: *mut IoReq,
    pub(crate) tail: *mut IoReq,
}
unsafe impl Send for IoReqFifo {}
impl Drop for IoReqFifo {
    fn drop(&mut self) {
        if !self.is_empty() {
            eprintln!("IoReqFifo::drop() called on an non empty list");
            std::process::abort();
        }
    }
}

impl IoReqFifo {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    pub(crate) unsafe fn pop_front(&mut self) -> Option<NonNull<IoReq>> {
        let old_head = self.head;
        if !old_head.is_null() {
            self.head = (*old_head).list_pop_next(Ordering::Relaxed);
            if self.head.is_null() {
                self.tail = std::ptr::null_mut();
            }
        }
        NonNull::new(old_head)
    }

    pub(crate) unsafe fn push_back(&mut self, mut new_tail: NonNull<IoReq>) {
        let new_tail = new_tail.as_mut();
        (*new_tail).list_set_next(std::ptr::null_mut(), Ordering::Relaxed);
        if self.tail.is_null() {
            self.head = new_tail;
        } else {
            (*self.tail).list_update_next(new_tail, Ordering::Relaxed);
        }
        self.tail = new_tail;
    }

    pub fn push_back_all(&mut self, other: &mut IoReqFifo) {
        if other.is_empty() {
            return;
        }
        if self.tail.is_null() {
            self.head = other.head;
        } else {
            unsafe { (*(self.tail)).list_update_next(other.head, Ordering::Relaxed) };
        }
        self.tail = other.tail;
        other.head = std::ptr::null_mut();
        other.tail = std::ptr::null_mut();
    }
}
impl Default for IoReqFifo {
    fn default() -> Self {
        Self {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple() {
        let mut a = IoReq::default();
        let mut b = IoReq::default();
        let mut c = IoReq::default();
        let mut d = IoReq::default();
        let pa = unsafe { NonNull::new_unchecked(&mut a as *mut IoReq) };
        let pb = unsafe { NonNull::new_unchecked(&mut b as *mut IoReq) };
        let pc = unsafe { NonNull::new_unchecked(&mut c as *mut IoReq) };
        let _pd = unsafe { NonNull::new_unchecked(&mut d as *mut IoReq) };

        let mut ready0 = IoReqFifo::new();
        assert!(unsafe { ready0.pop_front() }.is_none());
        assert!(ready0.is_empty());

        unsafe { ready0.push_back(pa) };
        assert!(!ready0.is_empty());

        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());

        unsafe { ready0.push_back(pa) };
        assert!(!ready0.is_empty());
        unsafe { ready0.push_back(pb) };
        assert!(!ready0.is_empty());
        unsafe { ready0.push_back(pc) };
        assert!(!ready0.is_empty());

        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pb);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pc);
        assert!(ready0.is_empty());

        let mut ready1 = IoReqFifo::new();
        unsafe { ready0.push_back(pa) };
        assert!(!ready0.is_empty());
        unsafe { ready0.push_back(pb) };
        unsafe { ready1.push_back(pc) };
        assert!(!ready1.is_empty());

        ready0.push_back_all(&mut ready1);
        assert!(!ready0.is_empty());
        assert!(ready1.is_empty());

        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pb);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pc);
        assert!(ready0.is_empty());

        unsafe { ready1.push_back(pa) };
        // println!(" * * * ready1: {ready1:?}");
        unsafe { ready1.push_back(pb) };
        // println!(" * * * ready1: {ready1:?}");
        unsafe { ready1.push_back(pc) };
        // println!(" * * * ready1: {ready1:?}");
        assert!(!ready1.is_empty());
        ready0.push_back_all(&mut ready1);
        assert!(!ready0.is_empty());
        assert!(ready1.is_empty());
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pb);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pc);
        assert!(ready0.is_empty());

        unsafe { ready0.push_back(pa) };
        assert!(!ready0.is_empty());
        ready0.push_back_all(&mut ready1);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());
    }
}
