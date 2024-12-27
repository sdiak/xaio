#![feature(associated_type_defaults)]

use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

pub trait SListNode: Sized {
    fn offset_of_link() -> usize;
    fn drop(ptr: Box<Self>);
}

#[repr(transparent)]
pub struct Uniq<T: Sized>(NonNull<T>);

impl<T: Sized> Uniq<T> {
    pub const LAYOUT: std::alloc::Layout = unsafe {
        std::alloc::Layout::from_size_align_unchecked(
            std::mem::size_of::<T>(),
            std::mem::align_of::<T>(),
        )
    };
    pub fn new(value: T) -> Option<Self> {
        let ptr = unsafe { std::alloc::alloc(Self::LAYOUT) } as *mut T;
        if !ptr.is_null() {
            unsafe { ptr.write(value) };
            Some(Self(unsafe { NonNull::new_unchecked(ptr) }))
        } else {
            None
        }
    }
}
impl<T: Sized> Deref for Uniq<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
impl<T: Sized> DerefMut for Uniq<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}
impl<T: Sized> Drop for Uniq<T> {
    fn drop(&mut self) {
        let ptr = self.0.as_ptr();
        unsafe {
            std::ptr::drop_in_place(ptr);
            std::alloc::dealloc(ptr as _, Self::LAYOUT);
        }
    }
}

pub struct SLink {
    next: AtomicUsize,
}

impl SLink {
    pub const fn new() -> SLink {
        Self {
            next: AtomicUsize::new(0),
        }
    }

    const IN_A_LIST_BIT: usize = 1usize;

    #[inline(always)]
    pub(crate) fn from<T: SListNode>(mut node: Box<T>) -> *mut SLink {
        let uptr = node.as_mut() as *mut T as usize + T::offset_of_link();
        std::mem::forget(node);
        uptr as _
    }
    #[inline(always)]
    pub(crate) fn into<T: SListNode>(link: *mut SLink) -> Box<T> {
        let uptr = link as usize - T::offset_of_link();
        unsafe { Box::from_raw(uptr as *mut T) }
    }

    #[inline(always)]
    pub(crate) fn into_ref_mut<'a, T: SListNode>(link: *mut SLink) -> Option<&'a mut T> {
        if link.is_null() {
            None
        } else {
            let uptr = link as usize - T::offset_of_link();
            Some(unsafe { &mut *(uptr as *mut T) })
        }
    }

    #[inline(always)]
    pub(crate) fn into_ref<'a, T: SListNode>(link: *const SLink) -> Option<&'a T> {
        if link.is_null() {
            None
        } else {
            let uptr = link as usize - T::offset_of_link();
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
