use std::{ptr, sync::atomic::Ordering};

use crate::Request;

pub struct RequestList {
    head: *mut Request,
}
impl Drop for RequestList {
    fn drop(&mut self) {
        debug_assert!(self.is_empty());
    }
}

impl RequestList {
    fn new() -> RequestList {
        RequestList {
            head: ptr::null_mut(),
        }
    }

    fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// O(1)
    unsafe fn push_front(&mut self, node: *mut Request) {
        debug_assert!(!node.is_null());
        (*node).list_set_next(self.head, Ordering::Relaxed);
        self.head = node;
    }
    /// O(1)
    unsafe fn pop_front(&mut self) -> *mut Request {
        if self.head.is_null() {
            ptr::null_mut()
        } else {
            let old_head = self.head;
            self.head = (*old_head).list_pop_next(Ordering::Relaxed);
            old_head
        }
    }
    fn contains(&self, node: *const Request) -> bool {
        if node.is_null() || !(unsafe { (*node).in_a_list() }) {
            return false;
        }
        let mut it: *const Request = self.head;
        while !it.is_null() {
            if it == node {
                return true;
            }
            it = unsafe { (*it).list_get_next(Ordering::Relaxed) };
        }
        false
    }
    unsafe fn remove(&mut self, node: *mut Request) -> bool {
        if node.is_null() || !(unsafe { (*node).in_a_list() }) {
            return false;
        }
        if node == self.head {
            self.pop_front();
            return true;
        }
        let mut prev: *mut Request = self.head;
        let mut it: *mut Request = unsafe { (*prev).list_get_next(Ordering::Relaxed) };
        while !it.is_null() {
            if it == node {
                unsafe {
                    (*prev).list_update_next(
                        (*it).list_pop_next(Ordering::Relaxed),
                        Ordering::Relaxed,
                    );
                }
                return true;
            }
            prev = it;
            it = unsafe { (*it).list_get_next(Ordering::Relaxed) };
        }
        false
    }

    /// O(n)
    unsafe fn push_back(&mut self, node: *mut Request) {
        debug_assert!(!node.is_null());
        // Ensures in a single list at a given time
        (*node).list_set_next(ptr::null_mut(), Ordering::Relaxed);
        if self.head.is_null() {
            self.head = node;
        } else {
            let mut prev: *mut Request = self.head;
            let mut prev_next = (*prev).list_get_next(Ordering::Relaxed);
            while !prev_next.is_null() {
                prev = prev_next as *mut Request;
                prev_next = (*prev).list_get_next(Ordering::Relaxed);
            }
            (*prev).list_update_next(node, Ordering::Relaxed);
        }
    }
    // /// O(n)
    // unsafe fn pop_back(&mut self) -> *mut Request {
    //     if self.head.is_null() {
    //         ptr::null_mut()
    //     } else {
    //         let mut prev: *mut Request = self.head;
    //         let mut prev_next = unsafe { (*prev).list_get_next(Ordering::Relaxed) };
    //         if prev_next.is_null() {
    //             self.head = prev_next;
    //             return prev;
    //         }
    //         let mut prev_next_next = unsafe { (*prev_next).list_get_next(Ordering::Relaxed) };
    //         while !prev_next_next.is_null() {
    //             prev = prev_next;
    //             prev_next = prev_next_next;
    //             prev_next_next = unsafe { (*prev_next).list_get_next(Ordering::Relaxed) };
    //         }
    //         (*prev).list_update_next(ptr::null_mut(), Ordering::Relaxed);
    //         (*prev_next).list_pop_next(Ordering::Relaxed);
    //         prev_next
    //     }
    // }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let mut l = RequestList::new();
        let mut a = Request::default();
        assert!(l.is_empty());
        assert!(!l.contains(ptr::null()));
        assert!(!l.contains(&a as *const Request));
        assert!(unsafe { l.pop_front() }.is_null());
        assert!(!a.in_a_list());
        unsafe { l.push_front(&mut a as *mut Request) };
        assert!(l.contains(&a as *const Request));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert_eq!(unsafe { l.pop_front() }, &mut a as *mut Request);
        assert!(l.is_empty());
        assert!(unsafe { l.pop_front() }.is_null());
        assert!(!a.in_a_list());
        assert!(!l.contains(&a as *const Request));
    }
    #[test]
    #[should_panic]
    fn test_push_front_null() {
        let mut l = RequestList::new();
        unsafe { l.push_front(ptr::null_mut()) };
    }

    #[test]
    fn test_push_back() {
        let mut l = RequestList::new();
        let mut a = Request::default();
        let mut b = Request::default();
        let mut c = Request::default();
        assert!(l.is_empty());
        assert!(!l.contains(ptr::null()));
        assert!(!l.contains(&a as *const Request));
        assert!(unsafe { l.pop_front() }.is_null());
        assert!(!a.in_a_list());
        unsafe { l.push_back(&mut a as *mut Request) };
        assert!(l.contains(&a as *const Request));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert_eq!(unsafe { l.pop_front() }, &mut a as *mut Request);
        assert!(l.is_empty());
        assert!(unsafe { l.pop_front() }.is_null());
        assert!(!a.in_a_list());
        assert!(!l.contains(&a as *const Request));
        assert!(!l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        unsafe { l.push_front(&mut a as *mut Request) };
        assert!(l.contains(&a as *const Request));
        assert!(!l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        unsafe { l.push_back(&mut b as *mut Request) };
        assert!(l.contains(&a as *const Request));
        assert!(l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        unsafe { l.push_back(&mut c as *mut Request) };
        assert!(l.contains(&a as *const Request) && a.in_a_list());
        assert!(l.contains(&b as *const Request) && b.in_a_list());
        assert!(l.contains(&c as *const Request) && c.in_a_list());

        assert_eq!(unsafe { l.pop_front() }, &mut a as *mut Request);
        assert!(!l.contains(&a as *const Request) && !a.in_a_list());
        assert_eq!(unsafe { l.pop_front() }, &mut b as *mut Request);
        assert!(!l.contains(&b as *const Request) && !b.in_a_list());
        assert_eq!(unsafe { l.pop_front() }, &mut c as *mut Request);
        assert!(!l.contains(&c as *const Request) && !c.in_a_list());
    }

    #[test]
    #[should_panic]
    fn test_push_back_null() {
        let mut l = RequestList::new();
        unsafe { l.push_front(ptr::null_mut()) };
    }

    #[test]
    fn test_two() {
        let mut l = RequestList::new();
        let mut a = Request::default();
        let mut b = Request::default();
        let mut l2 = RequestList::new();
        let mut c = Request::default();
        unsafe { l2.push_front(&mut c as *mut Request) };
        assert!(l.is_empty());
        assert!(unsafe { l.pop_front() }.is_null());
        assert!(!a.in_a_list());
        assert!(!b.in_a_list());
        assert!(c.in_a_list());
        assert!(!l.contains(&a as *const Request));
        assert!(!l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        unsafe { l.push_front(&mut a as *mut Request) };
        assert!(l.contains(&a as *const Request));
        assert!(!l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        unsafe { l.push_front(&mut b as *mut Request) };
        assert!(l.contains(&a as *const Request));
        assert!(l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert!(b.in_a_list());
        assert_eq!(unsafe { l.pop_front() }, &mut b as *mut Request);
        assert!(l.contains(&a as *const Request));
        assert!(!l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert!(!b.in_a_list());
        assert_eq!(unsafe { l.pop_front() }, &mut a as *mut Request);
        assert!(!l.contains(&a as *const Request));
        assert!(!l.contains(&b as *const Request));
        assert!(!l.contains(&c as *const Request));
        assert!(l.is_empty());
        assert!(unsafe { l.pop_front() }.is_null());
        assert!(!a.in_a_list());
        assert_eq!(unsafe { l2.pop_front() }, &mut c as *mut Request);
    }

    #[test]
    fn test_remove() {
        let mut l = RequestList::new();
        let mut a = Request::default();
        let mut b = Request::default();
        let mut l2 = RequestList::new();
        let mut c = Request::default();

        unsafe {
            l.push_front(&mut a as *mut Request);
            l.push_front(&mut b as *mut Request);
            l.push_front(&mut c as *mut Request);
            assert!(l.remove(&mut c as *mut Request));
            assert!(l.remove(&mut b as *mut Request));
            assert!(l.remove(&mut a as *mut Request));
            assert!(!l.remove(&mut a as *mut Request));
        }
        assert!(!a.in_a_list());
        assert!(!b.in_a_list());
        assert!(!c.in_a_list());

        unsafe {
            l.push_front(&mut a as *mut Request);
            l.push_front(&mut b as *mut Request);
            l.push_front(&mut c as *mut Request);
            assert!(a.in_a_list());
            assert!(b.in_a_list());
            assert!(c.in_a_list());
            assert!(l.remove(&mut a as *mut Request));
            assert!(l.remove(&mut b as *mut Request));
            assert!(l.remove(&mut c as *mut Request));

            l.push_front(&mut a as *mut Request);
            l.push_front(&mut b as *mut Request);
            l.push_front(&mut c as *mut Request);
            assert!(l.remove(&mut b as *mut Request));
            assert!(l.remove(&mut a as *mut Request));
            assert!(l.remove(&mut c as *mut Request));

            l.push_front(&mut a as *mut Request);
            l.push_front(&mut b as *mut Request);
            l.push_front(&mut c as *mut Request);
            assert!(l.remove(&mut b as *mut Request));
            assert!(l.remove(&mut c as *mut Request));
            assert!(l.remove(&mut a as *mut Request));

            l.push_front(&mut a as *mut Request);
            l.push_front(&mut b as *mut Request);
            l2.push_front(&mut c as *mut Request);
            assert!(l.remove(&mut a as *mut Request));
            assert!(!l.remove(&mut c as *mut Request));
            assert!(l.remove(&mut b as *mut Request));
            assert!(l2.remove(&mut c as *mut Request));
            assert!(!l.remove(ptr::null_mut()));
        }
    }
}
