use std::{ptr, sync::atomic::Ordering};

use crate::Sub;

pub struct SubList {
    head: *mut Sub,
}
impl Drop for SubList {
    fn drop(&mut self) {
        debug_assert!(self.is_empty());
    }
}

impl SubList {
    fn new() -> SubList {
        SubList {
            head: ptr::null_mut(),
        }
    }

    fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// O(1)
    fn push_front(&mut self, node: *mut Sub) {
        debug_assert!(!node.is_null());
        unsafe { (*node).list_set_next(self.head, Ordering::Relaxed) };
        self.head = node;
    }
    /// O(1)
    fn pop_front(&mut self) -> *mut Sub {
        if self.head.is_null() {
            ptr::null_mut()
        } else {
            let old_head = self.head;
            self.head = unsafe { (*old_head).list_pop_next(Ordering::Relaxed) };
            old_head
        }
    }
    fn contains(&self, node: *const Sub) -> bool {
        if node.is_null() || !(unsafe { (*node).in_a_list() }) {
            return false;
        }
        let mut it: *const Sub = self.head;
        while !it.is_null() {
            if it == node {
                return true;
            }
            it = unsafe { (*it).list_get_next(Ordering::Relaxed) };
        }
        false
    }
    fn remove(&mut self, node: *mut Sub) -> bool {
        if node.is_null() || !(unsafe { (*node).in_a_list() }) {
            return false;
        }
        if node == self.head {
            self.pop_front();
            return true;
        }
        let mut prev: *mut Sub = self.head;
        let mut it: *mut Sub = unsafe { (*prev).list_get_next(Ordering::Relaxed) };
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
    /*
    /// O(n)
    fn push_back(&mut self, node: *mut Sub) {
        debug_assert!(!node.is_null());
        unsafe {
            // Ensures in a single list at a given time
            (*node).list_set_next(ptr::null_mut(), Ordering::Relaxed);
            if self.head.is_null() {
                self.head = node;
            } else {
                let mut prev: *mut Sub = self.head;
                let mut prev_next = (*prev).list_get_next(Ordering::Relaxed);
                while !prev_next.is_null() {
                    prev = prev_next as *mut Sub;
                    prev_next = (*prev).list_get_next(Ordering::Relaxed);
                }
                (*prev).list_set_next(node, Ordering::Relaxed);
            }
        }
    }
    /// O(n)
    fn pop_back(&mut self) -> *mut Sub {
        if self.head.is_null() {
            ptr::null_mut()
        } else {
            let mut prev: *mut Sub = self.head;
            let mut prev_next = unsafe { (*prev).list_get_next(Ordering::Relaxed) };
            if prev_next.is_null() {
                self.head = prev_next;
                return prev;
            }
            let mut prev_next_next = unsafe { (*prev_next).list_get_next(Ordering::Relaxed) };
            while !prev_next_next.is_null() {
                prev = prev_next;
                prev_next = prev_next_next;
                prev_next_next = unsafe { (*prev_next).list_get_next(Ordering::Relaxed) };
            }
            unsafe { (*prev).list_set_next(ptr::null_mut(), Ordering::Relaxed); };
            prev_next
        }
    }*/
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let mut l = SubList::new();
        let mut a = Sub::default();
        assert!(l.is_empty());
        assert!(!l.contains(ptr::null()));
        assert!(!l.contains(&a as *const Sub));
        assert!(l.pop_front().is_null());
        assert!(!a.in_a_list());
        l.push_front(&mut a as *mut Sub);
        assert!(l.contains(&a as *const Sub));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert_eq!(l.pop_front(), &mut a as *mut Sub);
        assert!(l.is_empty());
        assert!(l.pop_front().is_null());
        assert!(!a.in_a_list());
        assert!(!l.contains(&a as *const Sub));
    }
    #[test]
    #[should_panic]
    fn test_push_front_null() {
        let mut l = SubList::new();
        l.push_front(ptr::null_mut());
    }

    #[test]
    fn test_two() {
        let mut l = SubList::new();
        let mut a = Sub::default();
        let mut b = Sub::default();
        let mut l2 = SubList::new();
        let mut c = Sub::default();
        l2.push_front(&mut c as *mut Sub);
        assert!(l.is_empty());
        assert!(l.pop_front().is_null());
        assert!(!a.in_a_list());
        assert!(!b.in_a_list());
        assert!(c.in_a_list());
        assert!(!l.contains(&a as *const Sub));
        assert!(!l.contains(&b as *const Sub));
        assert!(!l.contains(&c as *const Sub));
        l.push_front(&mut a as *mut Sub);
        assert!(l.contains(&a as *const Sub));
        assert!(!l.contains(&b as *const Sub));
        assert!(!l.contains(&c as *const Sub));
        l.push_front(&mut b as *mut Sub);
        assert!(l.contains(&a as *const Sub));
        assert!(l.contains(&b as *const Sub));
        assert!(!l.contains(&c as *const Sub));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert!(b.in_a_list());
        assert_eq!(l.pop_front(), &mut b as *mut Sub);
        assert!(l.contains(&a as *const Sub));
        assert!(!l.contains(&b as *const Sub));
        assert!(!l.contains(&c as *const Sub));
        assert!(!l.is_empty());
        assert!(a.in_a_list());
        assert!(!b.in_a_list());
        assert_eq!(l.pop_front(), &mut a as *mut Sub);
        assert!(!l.contains(&a as *const Sub));
        assert!(!l.contains(&b as *const Sub));
        assert!(!l.contains(&c as *const Sub));
        assert!(l.is_empty());
        assert!(l.pop_front().is_null());
        assert!(!a.in_a_list());
        assert_eq!(l2.pop_front(), &mut c as *mut Sub);
    }

    #[test]
    fn test_remove() {
        let mut l = SubList::new();
        let mut a = Sub::default();
        let mut b = Sub::default();
        let mut l2 = SubList::new();
        let mut c = Sub::default();

        l.push_front(&mut a as *mut Sub);
        l.push_front(&mut b as *mut Sub);
        l.push_front(&mut c as *mut Sub);
        assert!(l.remove(&mut c as *mut Sub));
        assert!(l.remove(&mut b as *mut Sub));
        assert!(l.remove(&mut a as *mut Sub));
        assert!(!l.remove(&mut a as *mut Sub));
        assert!(!a.in_a_list());
        assert!(!b.in_a_list());
        assert!(!c.in_a_list());

        l.push_front(&mut a as *mut Sub);
        l.push_front(&mut b as *mut Sub);
        l.push_front(&mut c as *mut Sub);
        assert!(a.in_a_list());
        assert!(b.in_a_list());
        assert!(c.in_a_list());
        assert!(l.remove(&mut a as *mut Sub));
        assert!(l.remove(&mut b as *mut Sub));
        assert!(l.remove(&mut c as *mut Sub));

        l.push_front(&mut a as *mut Sub);
        l.push_front(&mut b as *mut Sub);
        l.push_front(&mut c as *mut Sub);
        assert!(l.remove(&mut b as *mut Sub));
        assert!(l.remove(&mut a as *mut Sub));
        assert!(l.remove(&mut c as *mut Sub));

        l.push_front(&mut a as *mut Sub);
        l.push_front(&mut b as *mut Sub);
        l.push_front(&mut c as *mut Sub);
        assert!(l.remove(&mut b as *mut Sub));
        assert!(l.remove(&mut c as *mut Sub));
        assert!(l.remove(&mut a as *mut Sub));

        l.push_front(&mut a as *mut Sub);
        l.push_front(&mut b as *mut Sub);
        l2.push_front(&mut c as *mut Sub);
        assert!(l.remove(&mut a as *mut Sub));
        assert!(!l.remove(&mut c as *mut Sub));
        assert!(l.remove(&mut b as *mut Sub));
        assert!(l2.remove(&mut c as *mut Sub));
        assert!(!l.remove(ptr::null_mut()));
    }
}
