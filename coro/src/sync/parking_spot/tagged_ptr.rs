use std::{marker::PhantomData, ptr::NonNull};

pub(crate) struct TaggedPointer<T> {
    addr_and_tag: usize,
    _phantom: PhantomData<T>,
}

pub(crate) const TAGGED_POINTER_ALIGN: usize = 8;
pub(crate) const TAGGED_POINTER_MASK: usize = TAGGED_POINTER_ALIGN - 1;

impl<T> TaggedPointer<T> {
    #[inline(always)]
    fn to_usize(addr: Option<NonNull<T>>) -> usize {
        if addr.is_none() {
            0usize
        } else {
            addr.unwrap().as_ptr() as usize
        }
    }
    #[inline(always)]
    fn from_usize(addr: usize) -> Option<NonNull<T>> {
        NonNull::new(addr as *mut T)
    }
    pub(crate) fn new(tag: usize, addr: Option<NonNull<T>>) -> Self {
        let addr = TaggedPointer::to_usize(addr);
        debug_assert!(
            ((tag & TAGGED_POINTER_MASK) == tag) && ((addr & TAGGED_POINTER_MASK) == 0),
            "Alignement is respected"
        );
        Self {
            addr_and_tag: addr | tag,
            _phantom: PhantomData {},
        }
    }
    pub(crate) fn non_null(tag: usize, addr: NonNull<T>) -> Self {
        let addr = addr.as_ptr() as usize;
        debug_assert!(
            ((tag & TAGGED_POINTER_MASK) == tag) && ((addr & TAGGED_POINTER_MASK) == 0),
            "Alignement is respected"
        );
        Self {
            addr_and_tag: addr | tag,
            _phantom: PhantomData {},
        }
    }
    pub(crate) const fn null(tag: usize) -> Self {
        debug_assert!(
            (tag & TAGGED_POINTER_MASK) == tag,
            "tag fits inside std alignement"
        );
        Self {
            addr_and_tag: tag,
            _phantom: PhantomData {},
        }
    }
    #[inline(always)]
    pub(crate) fn tag(&self) -> usize {
        self.addr_and_tag & TAGGED_POINTER_MASK
    }
    #[inline(always)]
    pub(crate) fn addr(&self) -> Option<NonNull<T>> {
        TaggedPointer::from_usize(self.addr_and_tag & !TAGGED_POINTER_MASK)
    }
    #[inline(always)]
    pub(crate) fn set_addr(&mut self, addr: Option<NonNull<T>>) {
        let addr = TaggedPointer::to_usize(addr);
        debug_assert!((addr & TAGGED_POINTER_MASK) == 0, "Alignement is respected");
        self.addr_and_tag = self.tag() | addr;
    }
}
