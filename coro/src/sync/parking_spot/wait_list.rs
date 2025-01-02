use std::ptr::NonNull;

use super::Context;

pub struct WaitList {
    head: Option<NonNull<Context>>,
    tail: Option<NonNull<Context>>,
}

impl WaitList {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    fn pop_front_unchecked(&mut self) -> NonNull<Context> {
        let mut old_head = unsafe { self.head.unwrap_unchecked() };
        self.head = unsafe { old_head.as_mut() }.list_next.pop();
        if self.head.is_none() {
            self.tail = None
        }
        old_head
    }
    pub fn pop_front(&mut self) -> Option<NonNull<Context>> {
        if self.head.is_none() {
            None
        } else {
            Some(self.pop_front_unchecked())
        }
    }

    pub fn push_front(&mut self, mut new_head: NonNull<Context>) {
        debug_assert!(unsafe { new_head.as_mut() }.list_next.get().is_none());
        if let Some(head) = self.head {
            unsafe { new_head.as_mut() }.list_next.set_non_null(head);
        } else {
            self.tail = Some(new_head);
        }
        self.head = Some(new_head);
    }

    pub fn push_back(&mut self, mut new_tail: NonNull<Context>) {
        debug_assert!(unsafe { new_tail.as_mut() }.list_next.get().is_none());
        if let Some(mut tail) = self.tail {
            unsafe { tail.as_mut().list_next.set_non_null(new_tail) };
        } else {
            self.head = Some(new_tail);
        }
        self.tail = Some(new_tail);
    }

    pub fn retain<F>(&mut self, mut f: F) -> WaitList
    where
        F: FnMut(&mut Context) -> bool,
    {
        let mut removed = WaitList::new();

        // First deal with head removal
        while let Some(mut head) = self.head {
            if f(unsafe { head.as_mut() }) {
                break;
            } else {
                removed.push_back(self.pop_front_unchecked());
            }
        }
        // Are their any nodes left ?
        if self.head.is_none() {
            return removed;
        }
        // Process non-head nodes
        let mut prev = unsafe { self.head.unwrap_unchecked() };
        let mut it: Option<NonNull<Context>> = unsafe { prev.as_mut().list_next.get() };
        while let Some(mut node) = it {
            if !f(unsafe { node.as_mut() }) {
                unsafe { prev.as_mut().list_next.set(node.as_mut().list_next.pop()) };
                removed.push_back(node);
                if it == self.tail {
                    self.tail = Some(prev);
                }
                it = Some(prev);
            }
            prev = node;
            it = unsafe { prev.as_mut().list_next.get() };
        }
        removed
    }
}
