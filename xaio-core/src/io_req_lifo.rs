use std::{ptr::NonNull, sync::atomic::Ordering};

use crate::IoReq;

#[derive(Debug)]
pub struct IoReqLifo {
    pub(crate) head: *mut IoReq,
    pub(crate) tail: *mut IoReq,
}
unsafe impl Send for IoReqLifo {}
impl Drop for IoReqLifo {
    fn drop(&mut self) {
        if !self.is_empty() {
            eprintln!("IoReqLifo::drop() called on an non empty list");
            std::process::abort();
        }
    }
}

impl IoReqLifo {
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

    pub(crate) unsafe fn push_front(&mut self, mut new_head: NonNull<IoReq>) {
        let new_head = new_head.as_mut();
        (*new_head).list_set_next(self.head, Ordering::Relaxed);
        if self.head.is_null() {
            self.tail = new_head;
        }
        self.head = new_head;
    }

    pub fn push_front_all(&mut self, other: &mut IoReqLifo) {
        if other.is_empty() {
            return;
        }
        unsafe { (*(other.tail)).list_update_next(self.head, Ordering::Relaxed) };
        self.head = other.head;
        other.head = std::ptr::null_mut();
        other.tail = std::ptr::null_mut();
    }
}

impl Default for IoReqLifo {
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

        let mut ready0 = IoReqLifo::new();
        assert!(unsafe { ready0.pop_front() }.is_none());
        assert!(ready0.is_empty());

        unsafe { ready0.push_front(pa) };
        assert!(!ready0.is_empty());

        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());

        unsafe { ready0.push_front(pa) };
        assert!(!ready0.is_empty());
        unsafe { ready0.push_front(pb) };
        assert!(!ready0.is_empty());
        unsafe { ready0.push_front(pc) };
        assert!(!ready0.is_empty());

        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pc);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pb);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());

        let mut ready1 = IoReqLifo::new();
        unsafe { ready0.push_front(pa) };
        assert!(!ready0.is_empty());
        unsafe { ready0.push_front(pb) };
        unsafe { ready1.push_front(pc) };
        assert!(!ready1.is_empty());

        ready0.push_front_all(&mut ready1);
        assert!(!ready0.is_empty());
        assert!(ready1.is_empty());

        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pc);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pb);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());

        unsafe { ready1.push_front(pa) };
        // println!(" * * * ready1: {ready1:?}");
        unsafe { ready1.push_front(pb) };
        // println!(" * * * ready1: {ready1:?}");
        unsafe { ready1.push_front(pc) };
        // println!(" * * * ready1: {ready1:?}");
        assert!(!ready1.is_empty());
        ready0.push_front_all(&mut ready1);
        assert!(!ready0.is_empty());
        assert!(ready1.is_empty());
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pc);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pb);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());

        unsafe { ready0.push_front(pa) };
        assert!(!ready0.is_empty());
        ready0.push_front_all(&mut ready1);
        assert_eq!(unsafe { ready0.pop_front() }.unwrap(), pa);
        assert!(ready0.is_empty());
    }
}
