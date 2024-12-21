use std::{marker::PhantomData, sync::atomic::Ordering};

pub use super::NodeRef;
use super::{SLink, SListNode};

pub struct SList<T: SListNode> {
    pub(crate) head: *mut SLink,
    pub(crate) tail: *mut SLink,
    _phantom: PhantomData<T>,
}

impl<T: SListNode> SList<T> {
    /// Returns a new empty list
    pub const fn new() -> SList<T> {
        Self {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
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

    /// Removes and returns the front of the list when `!self.is_empty()`
    ///
    /// # Returns
    ///  * `Some(old_front)` the old front of the list when `!self.is_empty()`
    ///  * `None` when `self.is_empty()`
    /// # Complexity
    ///  * O(1)
    pub fn pop_front(&mut self) -> Option<Box<T>> {
        let old_head = self.head;
        if !old_head.is_null() {
            self.head = unsafe { (*old_head).list_pop_next(Ordering::Relaxed) };
            if self.head.is_null() {
                self.tail = std::ptr::null_mut();
            }
            Some(SLink::into::<T>(old_head))
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
                    unsafe {
                        (*it).list_update_next(
                            (*old_tail).list_pop_next(Ordering::Relaxed),
                            Ordering::Relaxed,
                        )
                    };
                    return Some(SLink::into::<T>(old_tail));
                }
                it = next;
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use std::{collections::LinkedList, ops::Deref};

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

        // let mut l = LinkedList::<IntNode>::new();
        // l.push_back(IntNode::new(0));
        // let f = l.front().unwrap();
        // let a = l.pop_back().unwrap();
        // println!("{}", a.val);
        // drop(a);
        // println!("{}", f.val);
        /*let mut a = IoReq::default();
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
        assert!(ready0.is_empty());*/
    }
}
