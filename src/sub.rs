use std::{cell::RefCell, default, ptr, sync::atomic::Ordering};

use crate::RingInner;

pub(super) const PENDING: i32 = i32::MIN;

#[repr(C)]
// #[derive(Debug)]
pub struct Sub {
    // prv__cp: *mut xcp_s,
    pub(crate) owner: RefCell<RingInner>,
    status: i32,
    flags_and_op_code: u32,
    list_next: std::sync::atomic::AtomicUsize,
}
impl Default for Sub {
    fn default() -> Self {
        unsafe { std::mem::MaybeUninit::zeroed().assume_init() }
    }
}

impl Sub {
    const IN_A_LIST_BIT: usize = 1usize;

    #[inline]
    pub fn in_a_list(&self) -> bool {
        (self.list_next.load(Ordering::Relaxed) & Sub::IN_A_LIST_BIT) != 0
    }
    #[inline]
    pub fn list_set_next(&mut self, next: *mut Sub, order: Ordering) {
        debug_assert!(!self.in_a_list());
        self.list_next
            .store((next as usize) | Sub::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub fn list_update_next(&mut self, next: *mut Sub, order: Ordering) {
        debug_assert!(self.in_a_list());
        self.list_next
            .store((next as usize) | Sub::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub fn list_get_next(&self, order: Ordering) -> *mut Sub {
        (self.list_next.load(order) & !Sub::IN_A_LIST_BIT) as *mut Sub
    }
    #[inline]
    pub fn list_pop_next(&self, order: Ordering) -> *mut Sub {
        let old_head = (self.list_next.load(order) & !Sub::IN_A_LIST_BIT) as *mut Sub;
        self.list_next.store(0usize, Ordering::Relaxed);
        old_head
    }
    /*
    pub fn set_status(self, status: i32) -> bool {
        if status == PENDING {
            panic!("Invalid status");
        }
        let r = self
            .status
            .compare_exchange(PENDING, status, Ordering::Release, Ordering::Relaxed);
        r.is_ok()
    }
    pub fn set_status_local(self, status: i32) -> bool {
        if status == PENDING {
            panic!("Invalid status");
        }
        if self.status.load(Ordering::Relaxed) == PENDING {
            self.status.store(status, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
    /// Cancel the Sub and consume it
    pub fn cancel(self) -> bool {
        let r = self.status.compare_exchange(
            PENDING,
            libc::ECANCELED,
            Ordering::Release,
            Ordering::Relaxed,
        );
        r.is_ok()
    }
    */
}
