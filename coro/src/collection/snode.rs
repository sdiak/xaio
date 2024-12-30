use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::ptr::Ptr;

pub trait SListNode: Sized {
    const OFFSET_OF_LINK: usize;
}

pub struct SLink {
    next: AtomicUsize,
}
impl Debug for SLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SLink").finish_non_exhaustive()
    }
}
/*
impl<T> From<Ptr<T>> for *mut SLink
where
    T: SListNode,
{
    fn from(mut node: Ptr<T>) -> Self {
        let uptr = node.as_ptr() as usize + T::OFFSET_OF_LINK;
        std::mem::forget(node);
        uptr as _
    }
}
impl<T> From<*mut SLink> for Ptr<T>
where
    T: SListNode,
{
    fn from(link: *mut SLink) -> Self {
        let uptr = link as usize - T::OFFSET_OF_LINK;
        unsafe { Ptr::from_raw(uptr as *mut T) }
    }
} */

impl SLink {
    pub const fn new() -> SLink {
        Self {
            next: AtomicUsize::new(0),
        }
    }

    const IN_A_LIST_BIT: usize = 1usize;

    #[inline(always)]
    pub(crate) fn from<T: SListNode>(mut node: Ptr<T>) -> *mut SLink {
        let uptr = node.as_mut() as *mut T as usize + T::OFFSET_OF_LINK;
        std::mem::forget(node);
        uptr as _
    }
    #[inline(always)]
    pub(crate) fn into<T: SListNode>(link: *mut SLink) -> Ptr<T> {
        let uptr = link as usize - T::OFFSET_OF_LINK;
        unsafe { Ptr::from_raw_unchecked(uptr as *mut T) }
    }

    #[inline(always)]
    pub(crate) fn into_ref_mut<'a, T: SListNode>(link: *mut SLink) -> Option<&'a mut T> {
        if link.is_null() {
            None
        } else {
            let uptr = link as usize - T::OFFSET_OF_LINK;
            Some(unsafe { &mut *(uptr as *mut T) })
        }
    }

    #[inline(always)]
    pub(crate) fn into_ref<'a, T: SListNode>(link: *const SLink) -> Option<&'a T> {
        if link.is_null() {
            None
        } else {
            let uptr = link as usize - T::OFFSET_OF_LINK;
            Some(unsafe { &*(uptr as *mut T) })
        }
    }

    #[inline]
    pub(crate) fn in_a_list(&self) -> bool {
        (self.next.load(Ordering::Relaxed) & SLink::IN_A_LIST_BIT) != 0
    }
    #[inline]
    pub(crate) fn list_set_next(&mut self, next: *const SLink, order: Ordering) {
        debug_assert!(!self.in_a_list());
        self.next
            .store((next as usize) | SLink::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub(crate) fn list_update_next(&mut self, next: *const SLink, order: Ordering) {
        debug_assert!(self.in_a_list());
        self.next
            .store((next as usize) | SLink::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub(crate) fn list_get_next(&self, order: Ordering) -> *mut SLink {
        (self.next.load(order) & !SLink::IN_A_LIST_BIT) as *mut SLink
    }
    #[inline]
    pub(crate) fn list_pop_next(&self, order: Ordering) -> *mut SLink {
        let old_next = (self.next.load(order) & !SLink::IN_A_LIST_BIT) as *mut SLink;
        self.next.store(0usize, Ordering::Relaxed);
        old_next
    }
}
