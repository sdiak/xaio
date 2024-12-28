use crate::{
    driver::{Driver, Sender},
    Ptr,
};

pub struct Request {}

pub struct Handle(Ptr<Request>);
pub type Callback = fn(Ptr<Request>) -> Option<Ptr<Request>>;

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        type CellType<T> = std::cell::RefCell<T>;
    } else {
        type CellType<T> = std::cell::UnsafeCell<T>;
    }
}

#[derive(Debug, Clone)]
pub struct CompletionPort<D: Driver>(
    std::rc::Rc<CellType<CpInner<D>>>,
    crate::PhantomUnsync,
    crate::PhantomUnsend,
);

#[derive(Debug)]
struct CpInner<D: Driver> {
    sender: D::Sender,
}

impl<D: Driver> CpInner<D> {
    fn new(driver: &D) -> Self {
        Self {
            sender: driver.sender(),
        }
    }
}

impl<D: Driver> CompletionPort<D> {
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

    #[inline(always)]
    pub fn submit(&self, mut req: Ptr<Request>) -> Handle {
        let hndl = Handle(unsafe { Ptr::from_raw_unchecked(req.as_ptr()) });
        self.inner().sender.submit(req);
        hndl
    }

    #[inline(always)]
    pub fn flush(&self) -> usize {
        self.inner().sender.flush()
    }
}
