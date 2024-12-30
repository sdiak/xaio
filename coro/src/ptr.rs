use std::{
    alloc::Layout,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[repr(transparent)]
pub struct Ptr<T: Sized>(usize, PhantomData<T>);

const ALLOCATED_TAG: usize = 1;

#[cfg(test)]
static FAIL_NEXT_ALLOC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
#[allow(non_snake_case)]
pub fn test__fail_next_alloc() {
    FAIL_NEXT_ALLOC.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            if FAIL_NEXT_ALLOC.load(std::sync::atomic::Ordering::Relaxed) {
                FAIL_NEXT_ALLOC.store(false, std::sync::atomic::Ordering::Relaxed);
                std::ptr::null_mut()
            } else {
                unsafe { std::alloc::alloc(layout) }
            }
        } else {
            unsafe { std::alloc::alloc(layout) }
        }
    }
}
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    std::alloc::dealloc(ptr, layout);
}

/// Safe unique raw pointer to `T` with optional allocation/deallocation or wrapping
impl<T: Sized> Ptr<T> {
    /// The layout of `T` instances
    pub const LAYOUT: std::alloc::Layout = unsafe {
        std::alloc::Layout::from_size_align_unchecked(
            std::mem::size_of::<T>(),
            std::mem::align_of::<T>(),
        )
    };

    /// Allocates a new `T`
    ///
    /// # Parameters
    ///  * `value` the value to move to the newly allocated memory
    ///
    /// # Returns
    ///  * `Some(Uniq<T>)` on success
    ///  * `None` when the system is out of memory
    #[inline]
    pub fn try_new(value: T) -> Option<Self> {
        let ptr = unsafe { alloc(Self::LAYOUT) } as *mut T;
        if !ptr.is_null() {
            unsafe { ptr.write(value) };
            Some(Self((ptr as usize) | ALLOCATED_TAG, PhantomData {}))
        } else {
            None
        }
    }

    pub fn new(value: T) -> Self {
        Self::try_new(value).expect("Out of memory")
    }

    /// Wraps an existing pointer
    ///
    /// # Parameters
    ///  * `raw` the data address (**must** remain valid until the end of `Some(self)` lifetime)
    ///
    /// # Returns
    ///  * `Some(Uniq<T>)` on success
    ///  * `None` when `raw.is_null()`
    ///
    /// # Safety
    ///  A wrapped pointer can not be dropped, you must consume it with `Uniq<T>::into_raw()`
    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Option<Self> {
        if !raw.is_null() {
            Some(unsafe { Self::from_raw_unchecked(raw) })
        } else {
            None
        }
    }

    /// Wraps an existing pointer
    ///
    /// # Parameters
    ///  * `raw` the data address (**must** remain valid until the end of `Some(self)` lifetime)
    ///
    /// # Returns
    ///  * `Some(Uniq<T>)` on success
    ///  * `None` when `raw.is_null()`
    ///
    /// # Safety
    ///  A wrapped pointer can not be dropped, you must consume it with `Uniq<T>::into_raw()`
    #[inline]
    pub unsafe fn from_raw_unchecked(raw: *mut T) -> Self {
        Self(raw as usize, PhantomData {})
    }

    pub unsafe fn from_raw_owned_unchecked(raw: *mut T) -> Self {
        Self((raw as usize) | ALLOCATED_TAG, PhantomData {})
    }

    /// Consumes `self` and returns it as a raw pointer.
    /// Panics when `self.memory_is_owned()`
    ///
    /// # Safety
    ///  Only usable for `Uniq<T>` built with `Uniq<T>::from_raw(...)`
    #[inline]
    pub unsafe fn into_raw(self) -> *mut T {
        assert!(!self.memory_is_owned());
        self.into_raw_unchecked()
    }

    pub unsafe fn clone_raw(&self) -> Ptr<T> {
        assert!(!self.memory_is_owned());
        Ptr::from_raw_unchecked(self.0 as _)
    }

    /// Consumes `self` and returns it as a raw pointer
    ///
    /// # Safety
    ///  When `self.memory_is_owned()` the caller should call `Ptr::from_raw_owned_unchecked()` later to allow droping
    #[inline]
    pub unsafe fn into_raw_unchecked(mut self) -> *mut T {
        let raw = self.as_mut_ptr();
        std::mem::forget(self);
        raw
    }

    /// Returns the address of the value
    ///
    /// # Safety
    ///  Borrow rules and lifetime must be handled by the caller
    #[inline(always)]
    pub unsafe fn as_mut_ptr(&mut self) -> *mut T {
        (self.0 & !ALLOCATED_TAG) as _
    }

    /// Returns the address of the value
    ///
    /// # Safety
    ///  Borrow rules and lifetime must be handled by the caller
    #[inline(always)]
    pub unsafe fn as_ptr(&self) -> *const T {
        (self.0 & !ALLOCATED_TAG) as _
    }

    /// Returns a mutable reference to the pointee
    #[inline(always)]
    pub fn as_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe { &mut *self.as_mut_ptr() }
    }

    /// Returns a reference to the pointee
    #[inline(always)]
    pub fn as_ref<'a>(&'a self) -> &'a T {
        let ptr: *const T = (self.0 & !ALLOCATED_TAG) as _;
        unsafe { &*ptr }
    }

    /// Returns `true` when `self` owns the memory
    #[inline(always)]
    pub fn memory_is_owned(&self) -> bool {
        (self.0 & ALLOCATED_TAG) != 0
    }

    pub(crate) unsafe fn deallocate_without_dropping(self) {
        let ptr: *mut T = (self.0 & !ALLOCATED_TAG) as _;
        dealloc(ptr as _, Self::LAYOUT);
        std::mem::forget(self);
    }
}

impl<T: Sized> Deref for Ptr<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<T: Sized> DerefMut for Ptr<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
impl<T: Sized> Drop for Ptr<T> {
    #[inline]
    fn drop(&mut self) {
        if (self.0 & ALLOCATED_TAG) != 0 {
            unsafe {
                let ptr = self.as_mut_ptr();
                std::ptr::drop_in_place(ptr);
                dealloc(ptr as _, Self::LAYOUT);
            }
        } else {
            log::warn!("Dropping a wrapping Uniq<T>")
        }
    }
}

impl<T: Sized> Debug for Ptr<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
    #[test]
    fn test_raw() {
        init();
        {
            let p = unsafe { Ptr::from_raw(std::ptr::null_mut() as *mut i32) };
            assert!(p.is_none());
        }
        let mut i: i32 = 0;
        {
            let mut p = unsafe { Ptr::from_raw(&mut i as *mut i32) }.unwrap();
            assert_eq!(*p, 0);
            *p = 1;
            assert_eq!(*p, 1);
            assert!(!p.memory_is_owned());
            assert_eq!(unsafe { p.into_raw() }, &mut i as *mut i32);
        }
        let mut i: i32 = 0;
        {
            let mut p = unsafe { Ptr::from_raw(&mut i as *mut i32) }.unwrap();
            assert_eq!(*p, 0);
            *p = 1;
            assert_eq!(*p, 1);
            assert!(!p.memory_is_owned());
            drop(p);
        }
        assert_eq!(i, 1);
    }
    #[test]
    fn test_alloc_failure() {
        init();
        test__fail_next_alloc();
        let p = Ptr::try_new(42);
        assert!(p.is_none());
    }

    #[test]
    fn test_alloc() {
        init();

        let p = Ptr::try_new(42).unwrap();
        assert!(*p == 42);
        assert!(p.memory_is_owned());
        assert_eq!(format!("{:?}", p), "42");
        drop(p);
    }
}
