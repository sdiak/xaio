use std::ptr::NonNull;

use super::Context;

pub(super) struct ContextList {
    head: Option<NonNull<Context>>,
    tail: Option<NonNull<Context>>,
}

impl ContextList {
    pub(super) fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub(super) fn push_front(&mut self, mut new_head: NonNull<Context>) {
        debug_assert!(unsafe { new_head.as_mut().list_prev.is_none() });
        unsafe { new_head.as_mut().list_next = self.head };
        if let Some(mut head) = self.head {
            unsafe { head.as_mut().list_prev = Some(new_head) };
        } else {
            self.tail = Some(new_head);
        }
        self.head = Some(new_head);
    }
    pub(super) fn pop_front(&mut self) -> Option<NonNull<Context>> {
        if let Some(mut old_head_) = self.head {
            let old_head = unsafe { old_head_.as_mut() };
            self.head = old_head.list_next;
            if let Some(mut new_head) = self.head {
                unsafe { new_head.as_mut() }.list_prev = None;
                old_head.list_next = None;
            } else {
                self.tail = None;
            }
            Some(old_head_)
        } else {
            None
        }
    }

    pub(super) fn push_back(&mut self, mut new_tail: NonNull<Context>) {
        debug_assert!(unsafe { new_tail.as_mut().list_next.is_none() });
        unsafe { new_tail.as_mut().list_prev = self.tail };
        if let Some(mut tail) = self.tail {
            unsafe { tail.as_mut().list_next = Some(new_tail) };
        } else {
            self.head = Some(new_tail)
        }
        self.tail = Some(new_tail);
    }
    pub(super) fn pop_back(&mut self) -> Option<NonNull<Context>> {
        if let Some(mut old_tail_) = self.tail {
            let old_tail = unsafe { old_tail_.as_mut() };
            self.tail = old_tail.list_prev;
            if let Some(mut new_tail) = self.tail {
                unsafe { new_tail.as_mut() }.list_next = None;
                old_tail.list_prev = None;
            } else {
                self.head = None;
            }
            Some(old_tail_)
        } else {
            None
        }
    }

    pub(super) fn remove(&mut self, mut cx: NonNull<Context>) {
        let cx = unsafe { cx.as_mut() };
        debug_assert!(cx.list_prev.is_some() || cx.list_next.is_some());
        if cx.list_prev.is_none() {
            self.pop_front();
        } else if cx.list_next.is_none() {
            self.pop_back();
        } else {
            let prev = unsafe { cx.list_prev.unwrap_unchecked() };
            let next = unsafe { cx.list_prev.unwrap_unchecked() };
            // prev.
        }
    }
}
