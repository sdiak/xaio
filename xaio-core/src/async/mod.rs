use std::{
    alloc::Layout,
    boxed::Box,
    mem::ManuallyDrop,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

mod deadline;
pub use deadline::AsyncDeadline;
mod socket;
pub use socket::*;

use crate::Status;

pub struct Driver {}
pub struct CompletionPort2 {
    now: AtomicU64,
}
impl CompletionPort2 {
    pub fn now(&self) -> u64 {
        self.now.load(Ordering::Relaxed)
    }
}

pub struct PollContext<'a> {
    now: u64,
    driver: &'a Driver,
    port: &'a CompletionPort2,
}

pub trait AsyncData: Sized {
    fn poll(&mut self, cx: &PollContext) -> Status;
}

pub type AsyncPoll<D: AsyncData> = fn(&mut D, &PollContext) -> Status;
type AsyncDrop<D: AsyncData> = unsafe fn(*mut D) -> ();

pub type Completion<D: AsyncData> = fn(D, Status);

struct AsyncInner<D: AsyncData> {
    vtable: &'static AsyncVTable,
    status: AtomicI32,
    data: D,
}

struct AsyncVTable {
    layout: Layout,
    poll: unsafe fn(*mut (), &PollContext) -> Status,
    drop: unsafe fn(*mut ()),
}
const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}
pub struct Noop {
    unused: usize,
}
impl AsyncData for Noop {
    fn poll(&mut self, _cx: &PollContext) -> Status {
        Status::new(0)
    }
}
impl<D: AsyncData> AsyncInner<D> {
    pub const VTABLE: AsyncVTable = AsyncVTable {
        layout: unsafe {
            Layout::from_size_align_unchecked(
                max(
                    std::mem::size_of::<AsyncInner<D>>(),
                    std::mem::size_of::<AsyncInner<Noop>>(),
                ),
                max(
                    std::mem::align_of::<AsyncInner<D>>(),
                    std::mem::align_of::<AsyncInner<Noop>>(),
                ),
            )
        },
        poll: AsyncInner::<D>::__poll,
        drop: AsyncInner::<D>::__drop,
    };

    unsafe fn __poll(thiz: *mut (), cx: &PollContext) -> Status {
        (unsafe { &mut *(thiz as *mut AsyncInner<D>) })
            .data
            .poll(cx)
    }

    unsafe fn __drop(thiz: *mut ()) {
        let _ = std::ptr::read(thiz as *mut AsyncInner<D>);
    }

    fn new(data: D) -> Self {
        Self {
            vtable: &AsyncInner::<D>::VTABLE,
            status: AtomicI32::new(Status::pending().value()),
            data,
        }
    }
}

#[repr(transparent)]
pub struct Async(ManuallyDrop<Box<AsyncInner<Noop>>>);

impl Drop for Async {
    fn drop(&mut self) {
        let thiz = self.0.as_mut() as *mut AsyncInner<Noop>;
        let vtable = unsafe { (*thiz).vtable };
        unsafe { (vtable.drop)(thiz as _) };
        unsafe { std::alloc::dealloc(thiz as _, vtable.layout) };
    }
}

impl Async {
    pub(crate) fn new<D: AsyncData>(poll: AsyncPoll<D>, data: D) -> Option<Self> {
        let thiz: *mut AsyncInner<D> =
            unsafe { std::alloc::alloc(AsyncInner::<D>::VTABLE.layout) } as _;
        if !thiz.is_null() {
            unsafe { thiz.write(AsyncInner::<D>::new(data)) };
            // let dropper = |thiz: AsyncPoll<D>| {
            //     drop(thiz);
            // };
            Some(Self(unsafe { ManuallyDrop::new(Box::from_raw(thiz as _)) }))
        } else {
            None
        }
    }

    pub(crate) fn poll(&mut self, cx: &PollContext) -> Status {
        if true {
            // TODO: check still pending
            unsafe { (self.0.vtable.poll)(self.0.as_mut() as *mut AsyncInner<Noop> as _, cx) }
        } else {
            Status::new(0)
        }
    }
}
