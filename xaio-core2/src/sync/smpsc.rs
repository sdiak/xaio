use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::sys::ThreadId;

use crate::collection::{SLink, SList, SListNode};

const PARK_BIT: usize = 1;
pub struct Queue<T: SListNode> {
    owner_thread_id: ThreadId,
    tail: AtomicUsize,
    _phantom: PhantomData<T>,
}
impl<T: SListNode> Debug for Queue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Queue")
            .field("owner_thread_id", &self.owner_thread_id)
            .finish_non_exhaustive()
    }
}

impl<T: SListNode> Queue<T> {
    pub fn new() -> Self {
        Self {
            owner_thread_id: ThreadId::current(),
            tail: AtomicUsize::new(0),
            _phantom: PhantomData::<T> {},
        }
    }

    #[inline(always)]
    pub fn owner_thread_id(&self) -> ThreadId {
        self.owner_thread_id
    }

    pub fn append(&self, other: &mut SList<T>) -> bool {
        let head = other.head;
        let tail = other.tail;
        if head.is_null() {
            return false;
        }
        other.head = std::ptr::null_mut();
        other.head = std::ptr::null_mut();
        let mut old_tail = self.tail.load(Ordering::Acquire);
        loop {
            unsafe { (*tail).list_update_next((old_tail & !PARK_BIT) as _, Ordering::Relaxed) };
            match self.tail.compare_exchange_weak(
                old_tail,
                head as usize,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return old_tail == PARK_BIT;
                }
                Err(t) => {
                    old_tail = t;
                }
            }
        }
    }

    pub fn park<F: FnOnce(&mut SList<T>) -> usize>(&self, f: F, dst: &mut SList<T>) -> usize {
        self.__check_current_thread();
        let mut len = self.__park_begin(dst);
        len += f(dst);
        len + self.__park_end(dst)
    }

    #[inline]
    // #[cfg_attr(coverage, coverage(off))]
    fn __check_current_thread(&self) {
        if self.owner_thread_id != ThreadId::current() {
            // Mostly for c-binding
            eprintln!(
                "xaio-core::collection::smpsc::Queue::park can only be called from the owner thread"
            );
            std::process::abort();
        }
    }

    #[inline(always)]
    // #[cfg_attr(coverage, coverage(off))]
    fn __check_park_bit(&self, old_tail: usize) {
        debug_assert!(
            old_tail != PARK_BIT,
            "The park-bit can not be set at this stage"
        );
    }

    fn __park_begin(&self, dst: &mut SList<T>) -> usize {
        // println!("old_tail={}", self.tail.load(Ordering::Relaxed));
        let old_tail = self.tail.swap(PARK_BIT, Ordering::Acquire);
        if old_tail == 0 {
            0
        } else {
            // println!(" old_tail={old_tail}");
            self.__check_park_bit(old_tail);
            Self::__reverse_list(old_tail as _, dst)
        }
    }

    fn __park_end(&self, dst: &mut SList<T>) -> usize {
        let old_tail = self.tail.swap(0, Ordering::Acquire);
        if old_tail <= PARK_BIT {
            0
        } else {
            Self::__reverse_list(old_tail as _, dst)
        }
    }

    fn __reverse_list(src_tail: *mut SLink, dst: &mut SList<T>) -> usize {
        let mut len = 0usize;
        let tail: *mut SLink = src_tail;
        let mut head = src_tail;
        let mut prev = std::ptr::null_mut::<SLink>();
        while !head.is_null() {
            len += 1;
            let next = unsafe { (*head).list_get_next(Ordering::Relaxed) };
            unsafe { (*head).list_update_next(prev, Ordering::Relaxed) };
            prev = head;
            head = next;
        }
        dst.append(&mut SList::<T> {
            head: prev,
            tail,
            _phantom: PhantomData::<T> {},
        });
        len
    }
}
