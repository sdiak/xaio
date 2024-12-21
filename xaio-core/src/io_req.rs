use std::sync::atomic::Ordering;

pub struct IoReq {
    list_next: std::sync::atomic::AtomicUsize,
}
impl Default for IoReq {
    fn default() -> Self {
        unsafe { std::mem::zeroed::<Self>() }
    }
}

impl IoReq {
    const IN_A_LIST_BIT: usize = 1usize;

    #[inline]
    pub(crate) fn in_a_list(&self) -> bool {
        (self.list_next.load(Ordering::Relaxed) & IoReq::IN_A_LIST_BIT) != 0
    }
    #[inline]
    pub(crate) fn list_set_next(&mut self, next: *const IoReq, order: Ordering) {
        debug_assert!(!self.in_a_list());
        self.list_next
            .store((next as usize) | IoReq::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub(crate) fn list_update_next(&mut self, next: *const IoReq, order: Ordering) {
        debug_assert!(self.in_a_list());
        self.list_next
            .store((next as usize) | IoReq::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub(crate) fn list_get_next(&self, order: Ordering) -> *mut IoReq {
        (self.list_next.load(order) & !IoReq::IN_A_LIST_BIT) as *mut IoReq
    }
    #[inline]
    pub(crate) fn list_pop_next(&self, order: Ordering) -> *mut IoReq {
        let old_next = (self.list_next.load(order) & !IoReq::IN_A_LIST_BIT) as *mut IoReq;
        self.list_next.store(0usize, Ordering::Relaxed);
        old_next
    }
}
