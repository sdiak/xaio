use crate::{
    driver::{self, Driver, DriverTrait},
    Handle, Ptr, Request,
};

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        type CellType<T> = std::cell::RefCell<T>;
    } else {
        type CellType<T> = std::cell::UnsafeCell<T>;
    }
}

#[derive(Debug, Clone)]
pub struct CompletionPort(
    std::rc::Rc<CellType<CpInner>>,
    crate::PhantomUnsync,
    crate::PhantomUnsend,
);

/// The reference time for `CompletionPort::now()`
pub static EPOCH: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

#[derive(Debug)]
struct CpInner {
    driver: Driver,
    epoch: std::time::Instant,
    cached_now: u64,
    buffer: crate::collection::SList<crate::Request>,
    buffer_len: usize,
}

impl CpInner {
    fn new(driver: Driver, epoch: std::time::Instant) -> Self {
        Self {
            driver,
            epoch,
            cached_now: epoch.elapsed().as_millis() as _,
            buffer: crate::collection::SList::new(),
            buffer_len: 0,
        }
    }
}

impl CompletionPort {
    pub fn new(driver: Driver) -> Self {
        Self(
            std::rc::Rc::new(CellType::new(CpInner::new(driver, *EPOCH))),
            crate::PhantomUnsync {},
            crate::PhantomUnsend {},
        )
    }
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            #[inline(always)]
            fn inner_mut(&self) -> std::cell::RefMut<'_, CpInner> {
                self.0.borrow_mut()
            }
            #[inline(always)]
            fn inner(&self) -> std::cell::Ref<'_, CpInner> {
                self.0.borrow()
            }
        } else {
            #[inline(always)]
            fn inner_mut(&self) -> &mut CpInner {
                unsafe { &mut *self.0.get() }
            }
            #[inline(always)]
            fn inner(&self) -> &CpInner {
                unsafe { &*self.0.get() }
            }
        }
    }

    /// Returns the cached number of milliseconds since `completion_port::EPOCH`
    #[inline(always)]
    pub fn now(&self) -> u64 {
        self.inner().cached_now
    }

    /// Update and returns the cached number of milliseconds since `completion_port::EPOCH`
    #[inline(always)]
    pub fn update_now(&self) -> u64 {
        let mut inner = self.inner_mut();
        inner.cached_now = inner.epoch.elapsed().as_millis() as _;
        inner.cached_now
    }

    #[inline(always)]
    pub fn submit(&self, mut req: Ptr<Request>) -> Handle {
        let hndl = Handle::new(&mut req);
        {
            let mut inner = self.inner_mut();
            inner.buffer.push_front(req);
            inner.buffer_len += 1;
        }
        hndl
    }

    #[inline(always)]
    pub fn flush(&self) -> usize {
        let flushed = self.inner().buffer_len;
        if flushed > 0 {
            let mut inner = self.inner_mut();
            inner.buffer_len = 0;
            self.inner().driver.submit(&mut inner.buffer);
        }
        flushed
    }
}
