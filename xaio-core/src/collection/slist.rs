use std::{marker::PhantomData, sync::atomic::Ordering};

use super::{SLink, SListNode};

pub struct SList<T: SListNode> {
    pub(crate) head: *mut SLink,
    pub(crate) tail: *mut SLink,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T: SListNode> Drop for SList<T> {
    #[inline(always)]
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: SListNode> SList<T> {
    /// Returns a new empty list
    pub const fn new() -> Self {
        Self {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
            _phantom: PhantomData::<T> {},
        }
    }
    /// Returns a new list with the given node
    pub fn from_node(node: Box<T>) -> Self {
        let node: *mut SLink = SLink::from::<T>(node);
        unsafe { (*node).list_set_next(std::ptr::null_mut(), Ordering::Relaxed) };
        Self {
            head: node,
            tail: node,
            _phantom: PhantomData::<T> {},
        }
    }

    #[inline(always)]
    /// Returns `true` if the list is empty
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    #[inline(always)]
    /// # Returns
    ///  * `Some(front)` the front of the list when `!self.is_empty()`
    ///  * `None` when `self.is_empty()`
    pub fn front<'a>(&'a self) -> Option<&'a T> {
        SLink::into_ref::<'a, T>(self.head)
    }

    #[inline(always)]
    /// # Returns
    ///  * `Some(back)` the back of the list when `!self.is_empty()`
    ///  * `None` when `self.is_empty()`
    pub fn back<'a>(&'a self) -> Option<&'a T> {
        SLink::into_ref::<'a, T>(self.tail)
    }

    #[inline(always)]
    fn clear(&mut self) {
        if !self.head.is_null() {
            self.clear_non_empty();
        }
    }

    #[inline(never)]
    fn clear_non_empty(&mut self) {
        while !self.head.is_null() {
            let to_drop = self.head;
            self.head = unsafe { (*to_drop).list_pop_next(Ordering::Relaxed) };
            T::drop(SLink::into::<T>(to_drop));
        }
        self.tail = std::ptr::null_mut();
    }

    /// Adds a node at the front of the list
    ///
    /// # Arguments
    ///  * `new_front` the new front of the list
    /// # Complexity
    ///  * O(1)
    pub fn push_front(&mut self, new_front: Box<T>) {
        let new_head: *mut SLink = SLink::from::<T>(new_front);
        unsafe { (*new_head).list_set_next(self.head, Ordering::Relaxed) };
        if self.head.is_null() {
            self.tail = new_head;
        }
        self.head = new_head;
    }

    fn pop_front_unchecked(&mut self) -> Box<T> {
        let old_head = self.head;
        self.head = unsafe { (*old_head).list_pop_next(Ordering::Relaxed) };
        if self.head.is_null() {
            self.tail = std::ptr::null_mut();
        }
        SLink::into::<T>(old_head)
    }
    /// Removes and returns the front of the list when `!self.is_empty()`
    ///
    /// # Returns
    ///  * `Some(old_front)` the old front of the list when `!self.is_empty()`
    ///  * `None` when `self.is_empty()`
    /// # Complexity
    ///  * O(1)
    pub fn pop_front(&mut self) -> Option<Box<T>> {
        if !self.head.is_null() {
            Some(self.pop_front_unchecked())
        } else {
            None
        }
    }

    /// Adds a node at the back of the list
    ///
    /// # Arguments
    ///  * `new_back` the new back of the list
    /// # Complexity
    ///  * O(1)
    pub fn push_back(&mut self, new_back: Box<T>) {
        let new_tail = SLink::from::<T>(new_back);
        unsafe { (*new_tail).list_set_next(std::ptr::null_mut(), Ordering::Relaxed) };
        if self.tail.is_null() {
            self.head = new_tail;
        } else {
            unsafe { (*self.tail).list_update_next(new_tail, Ordering::Relaxed) };
        }
        self.tail = new_tail;
    }

    pub fn swap(&mut self, other: &mut SList<T>) {
        let other_tail = other.tail;
        let other_head = other.head;
        other.tail = self.tail;
        other.head = self.head;
        self.tail = other_tail;
        self.head = other_head;
    }

    pub fn prepend(&mut self, other: &mut SList<T>) {
        if other.is_empty() {
            return;
        }
        if self.tail.is_null() {
            self.tail = other.tail;
        } else {
            unsafe { (*other.tail).list_update_next(self.head, Ordering::Relaxed) };
        }
        self.head = other.head;
        other.head = std::ptr::null_mut();
        other.tail = std::ptr::null_mut();
    }

    pub fn append(&mut self, other: &mut SList<T>) {
        if other.is_empty() {
            return;
        }
        if self.tail.is_null() {
            self.head = other.head;
        } else {
            unsafe { (*self.tail).list_update_next(other.head, Ordering::Relaxed) };
        }
        self.tail = other.tail;
        other.head = std::ptr::null_mut();
        other.tail = std::ptr::null_mut();
    }

    /// Removes and returns the back of the list when `!self.is_empty()`
    ///
    /// # Returns
    ///  * `Some(old_back)` the old back of the list when `!self.is_empty()`
    ///  * `None` when `self.is_empty()`
    /// # Complexity
    ///  * O(n)
    pub fn pop_back(&mut self) -> Option<Box<T>> {
        let old_tail = self.tail;
        if !old_tail.is_null() {
            let mut it = self.head;
            if it == old_tail {
                self.head = std::ptr::null_mut();
                self.tail = std::ptr::null_mut();
                unsafe { (*old_tail).list_pop_next(Ordering::Relaxed) };
                return Some(SLink::into::<T>(old_tail));
            }
            loop {
                let next = unsafe { (*it).list_get_next(Ordering::Relaxed) };
                if next == old_tail {
                    unsafe { (*it).list_update_next(std::ptr::null_mut(), Ordering::Relaxed) };
                    self.tail = it;
                    return Some(SLink::into::<T>(old_tail));
                }
                it = next;
            }
        }
        None
    }

    pub fn iter<'a>(&'a self) -> SListIter<'a, T> {
        SListIter {
            pos: self.head,
            _phantom: PhantomData {},
        }
    }

    pub fn retain<F>(&mut self, mut f: F) -> SList<T>
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut removed = SList::new();

        // First deal with head removal
        while let Some(head) = SLink::into_ref_mut::<T>(self.head) {
            if f(head) {
                break;
            } else {
                removed.push_back(self.pop_front_unchecked());
            }
        }
        // Are their any nodes left ?
        if self.head.is_null() {
            self.tail = std::ptr::null_mut();
            return removed;
        }
        // Process non-head nodes
        let mut prev = self.head;
        let mut it = unsafe { (*prev).list_get_next(Ordering::Relaxed) };
        while let Some(mut node) = SLink::into_ref_mut::<T>(it) {
            if !f(node) {
                unsafe {
                    (*prev)
                        .list_update_next((*it).list_pop_next(Ordering::Relaxed), Ordering::Relaxed)
                };
                removed.push_back(unsafe { Box::from_raw(node as _) });
                if it == self.tail {
                    self.tail = prev;
                }
                it = prev;
            }
            prev = it;
            it = unsafe { (*it).list_get_next(Ordering::Relaxed) };
        }
        removed
    }
}

pub struct SListIter<'a, T: SListNode> {
    pub(crate) pos: *const SLink,
    pub(crate) _phantom: PhantomData<&'a T>,
}
impl<'a, T: SListNode> Iterator for SListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos.is_null() {
            None
        } else {
            let current = self.pos;
            self.pos = unsafe { (*current).list_get_next(Ordering::Relaxed) };
            SLink::into_ref::<'a, T>(current)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct IntNode {
        pub val: i32,
        link: SLink,
    }
    impl Drop for IntNode {
        fn drop(&mut self) {
            println!("Drop: {:#x}", self as *mut Self as usize);
        }
    }
    impl IntNode {
        fn new(val: i32) -> Self {
            Self {
                val,
                link: SLink::new(),
            }
        }
    }
    impl SListNode for IntNode {
        fn offset_of_link() -> usize {
            core::mem::offset_of!(IntNode, link)
        }
        fn drop(ptr: Box<Self>) {
            drop(ptr);
        }
    }

    #[test]
    fn test_simple() {
        let mut a = Box::<IntNode>::new(IntNode::new(0));
        let mut b = Box::<IntNode>::new(IntNode::new(1));
        let mut c = Box::<IntNode>::new(IntNode::new(2));
        let mut d = Box::<IntNode>::new(IntNode::new(3));

        let mut list = SList::<IntNode>::new();

        assert!(list.is_empty());
        assert!(list.pop_front().is_none());
        assert!(list.pop_back().is_none());

        list.push_back(a);
        assert!(!list.is_empty());
        a = list.pop_back().unwrap();
        assert_eq!(a.val, 0);
        assert!(list.pop_front().is_none());

        list.push_back(a);
        assert!(!list.is_empty());
        a = list.pop_front().unwrap();
        assert_eq!(a.val, 0);
        assert!(list.pop_back().is_none());

        list.push_back(a);
        assert!(!list.is_empty());
        assert_eq!(list.front().unwrap().val, 0);
        assert_eq!(list.back().unwrap().val, 0);
        let bor = list.front();
        let _ = bor;
        a = list.pop_front().unwrap();
        assert_eq!(a.val, 0);

        list.push_back(a);
        list.push_front(d);
        list.push_back(b);
        list.push_front(c);

        assert!(list.pop_front().unwrap().val == 2);
        assert!(list.pop_front().unwrap().val == 3);
        assert!(list.pop_front().unwrap().val == 0);
        assert!(list.pop_front().unwrap().val == 1);

        assert!(list.is_empty());
        assert!(list.front().is_none());
        assert!(list.back().is_none());
    }

    #[test]
    fn test_move() {
        let a = Box::<IntNode>::new(IntNode::new(0));
        let b = Box::<IntNode>::new(IntNode::new(1));
        let c = Box::<IntNode>::new(IntNode::new(2));
        let d = Box::<IntNode>::new(IntNode::new(3));
        let mut list = SList::<IntNode>::from_node(a);

        assert!(!list.is_empty());
        assert_eq!(list.front().unwrap().val, 0);
        assert_eq!(list.back().unwrap().val, 0);

        list.push_back(b);
        list.push_back(c);
        list.push_back(d);

        let mut list2 = SList::<IntNode>::from_node(Box::<IntNode>::new(IntNode::new(-1)));
        list2.append(&mut list);
        assert!(list.is_empty());
        list2.append(&mut SList::<IntNode>::new());

        assert!(list2.pop_back().unwrap().val == 3);
        assert!(list2.pop_back().unwrap().val == 2);
        assert!(list2.pop_back().unwrap().val == 1);
        assert!(list2.pop_back().unwrap().val == 0);
        assert!(list2.pop_back().unwrap().val == -1);

        list.push_back(Box::<IntNode>::new(IntNode::new(1)));
        list.push_back(Box::<IntNode>::new(IntNode::new(2)));
        list.push_back(Box::<IntNode>::new(IntNode::new(3)));
        let mut list2 = SList::<IntNode>::from_node(Box::<IntNode>::new(IntNode::new(-1)));
        list2.push_back(Box::<IntNode>::new(IntNode::new(0)));
        list.prepend(&mut list2);
        list.prepend(&mut SList::<IntNode>::new());
        assert!(list2.is_empty());
        let mut list3 = SList::<IntNode>::new();
        assert!(list3.is_empty());
        list3.prepend(&mut list);
        assert!(list.is_empty());
        assert!(!list3.is_empty());
        list3.swap(&mut list);
        assert!(!list.is_empty());
        assert!(list3.is_empty());

        assert!(list.pop_back().unwrap().val == 3);
        assert!(list.pop_back().unwrap().val == 2);
        assert!(list.pop_back().unwrap().val == 1);
        assert!(list.pop_back().unwrap().val == 0);
        assert!(list.pop_back().unwrap().val == -1);

        let mut list = SList::<IntNode>::from_node(Box::<IntNode>::new(IntNode::new(42)));
        let mut list2 = SList::<IntNode>::new();
        assert!(!list.is_empty());
        assert!(list2.is_empty());
        list2.append(&mut list);
        assert!(list.is_empty());
        assert!(!list2.is_empty());
        assert!(list2.pop_back().unwrap().val == 42);
        assert!(list2.is_empty());
    }

    #[test]
    fn test_push_back() {
        let mut a = Box::<IntNode>::new(IntNode::new(0));
        let mut b = Box::<IntNode>::new(IntNode::new(1));
        let mut c = Box::<IntNode>::new(IntNode::new(2));
        let mut d = Box::<IntNode>::new(IntNode::new(3));
        let mut list = SList::<IntNode>::new();

        assert!(list.is_empty());
        assert!(list.pop_front().is_none());
        assert!(list.pop_back().is_none());

        list.push_back(b);
        list.push_back(c);
        list.push_back(d);
        list.push_front(a);

        assert!(list.pop_back().unwrap().val == 3);
        assert!(list.pop_back().unwrap().val == 2);
        assert!(list.pop_back().unwrap().val == 1);
        assert!(list.pop_back().unwrap().val == 0);
        list.push_back(Box::<IntNode>::new(IntNode::new(0)));
        list.push_back(Box::<IntNode>::new(IntNode::new(1)));
        assert!(list.pop_back().unwrap().val == 1);
        assert!(list.pop_back().unwrap().val == 0);

        assert!(list.is_empty());
        list.push_back(Box::<IntNode>::new(IntNode::new(0)));
        list.push_back(Box::<IntNode>::new(IntNode::new(1)));
        assert!(!list.is_empty());
        list.clear();
        assert!(list.is_empty());
        list.push_front(Box::<IntNode>::new(IntNode::new(0)));
        list.push_front(Box::<IntNode>::new(IntNode::new(1)));
        drop(list);
    }
}
