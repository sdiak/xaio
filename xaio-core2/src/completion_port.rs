use crate::{
    driver::{DriverTrait, Sender},
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
pub struct CompletionPort<D: DriverTrait>(
    std::rc::Rc<CellType<CpInner<D>>>,
    crate::PhantomUnsync,
    crate::PhantomUnsend,
);

/// The reference time for `CompletionPort::now()`
pub static EPOCH: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

#[derive(Debug)]
struct CpInner<D: DriverTrait> {
    sender: D::Sender,
    epoch: std::time::Instant,
    cached_now: u64,
}

impl<D: DriverTrait> CpInner<D> {
    fn new(driver: &D, epoch: std::time::Instant) -> Self {
        Self {
            sender: driver.sender(),
            epoch,
            cached_now: epoch.elapsed().as_millis() as _,
        }
    }
}

impl<D: DriverTrait> CompletionPort<D> {
    pub fn new(driver: &D) -> Self {
        Self(
            std::rc::Rc::new(CellType::new(CpInner::new(driver, *EPOCH))),
            crate::PhantomUnsync {},
            crate::PhantomUnsend {},
        )
    }
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            #[inline(always)]
            fn inner_mut(&self) -> std::cell::RefMut<'_, CpInner<D>> {
                self.0.borrow_mut()
            }
            #[inline(always)]
            fn inner(&self) -> std::cell::Ref<'_, CpInner<D>> {
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
        self.inner().sender.submit(req);
        hndl
    }

    #[inline(always)]
    pub fn flush(&self) -> usize {
        self.inner().sender.flush()
    }
}
