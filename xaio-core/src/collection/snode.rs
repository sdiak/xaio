use std::{pin::Pin, sync::atomic::Ordering};

pub trait SListNode: Sized {
    fn offset_of_link() -> usize;
    // type Thiz;
    // unsafe fn as_intrusive<'a>(&'a self) -> &'a SListIntrusive;
    // unsafe fn as_mut_intrusive<'a>(&'a mut self) -> &'a mut SListIntrusive;
    // unsafe fn from_instrusive<'a>(i: &'a SListIntrusive) -> &'a Self::Thiz;
    // unsafe fn from_mut_instrusive<'a>(i: &'a mut SListIntrusive) -> &'a mut Self::Thiz;
}

pub struct SListLink {
    next: std::sync::atomic::AtomicUsize,
}

impl SListLink {
    const IN_A_LIST_BIT: usize = 1usize;

    #[inline(always)]
    pub(crate) fn from<T: SListNode>(mut node: Box<T>) -> *mut SListLink {
        let uptr = node.as_mut() as *mut T as usize + T::offset_of_link();
        std::mem::forget(node);
        uptr as _
    }
    #[inline(always)]
    pub(crate) fn into<T: SListNode>(link: *mut SListLink) -> Box<T> {
        let uptr = link as usize - T::offset_of_link();
        unsafe { Box::from_raw(uptr as *mut T) }
    }

    #[inline]
    pub(crate) fn in_a_list(&self) -> bool {
        (self.next.load(Ordering::Relaxed) & SListLink::IN_A_LIST_BIT) != 0
    }
    #[inline]
    pub(crate) fn list_set_next(&mut self, next: *const SListLink, order: Ordering) {
        debug_assert!(!self.in_a_list());
        self.next
            .store((next as usize) | SListLink::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub(crate) fn list_update_next(&mut self, next: *const SListLink, order: Ordering) {
        debug_assert!(self.in_a_list());
        self.next
            .store((next as usize) | SListLink::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub(crate) fn list_get_next(&self, order: Ordering) -> *mut SListLink {
        (self.next.load(order) & !SListLink::IN_A_LIST_BIT) as *mut SListLink
    }
    #[inline]
    pub(crate) fn list_pop_next(&self, order: Ordering) -> *mut SListLink {
        let old_next = (self.next.load(order) & !SListLink::IN_A_LIST_BIT) as *mut SListLink;
        self.next.store(0usize, Ordering::Relaxed);
        old_next
    }
}
