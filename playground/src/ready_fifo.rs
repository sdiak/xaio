use std::{ptr::NonNull, sync::atomic::Ordering};

use crate::Request;

#[derive(Debug)]
pub struct ReadyFifo {
    pub(crate) head: *mut Request,
    pub(crate) tail: *mut Request,
}
unsafe impl Send for ReadyFifo {}
impl Drop for ReadyFifo {
    fn drop(&mut self) {
        debug_assert!(self.is_empty());
    }
}

impl ReadyFifo {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    pub(crate) unsafe fn pop_front(&mut self) -> Option<NonNull<Request>> {
        let old_head = self.head;
        if !old_head.is_null() {
            self.head = (*old_head).list_pop_next(Ordering::Relaxed);
            if self.head.is_null() {
                self.tail = std::ptr::null_mut();
            }
        }
        NonNull::new(old_head)
    }

    pub(crate) unsafe fn push_back(&mut self, mut new_tail: NonNull<Request>) {
        let new_tail = new_tail.as_mut();
        (*new_tail).list_set_next(std::ptr::null_mut(), Ordering::Relaxed);
        if self.tail.is_null() {
            self.head = new_tail;
        } else {
            (*self.tail).list_update_next(new_tail, Ordering::Relaxed);
        }
        self.tail = new_tail;
    }

    pub fn push_back_all(&mut self, other: &mut ReadyFifo) {
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
impl Default for ReadyFifo {
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
        let mut a = Request::default();
        let mut b = Request::default();
        let mut c = Request::default();
        let mut d = Request::default();
        let pa = unsafe { NonNull::new_unchecked(&mut a as *mut Request) };
        let pb = unsafe { NonNull::new_unchecked(&mut b as *mut Request) };
        let pc = unsafe { NonNull::new_unchecked(&mut c as *mut Request) };
        let _pd = unsafe { NonNull::new_unchecked(&mut d as *mut Request) };

        let mut ready0 = ReadyFifo::new();
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

        let mut ready1 = ReadyFifo::new();
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
