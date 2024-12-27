use std::{
    future::Future,
    io::{Error, ErrorKind, Result},
    ptr::NonNull,
    sync::{
        atomic::{AtomicU64, Ordering},
        LazyLock,
    },
    time::Instant,
};

use xaio_core::collection::SList;

use crate::Task;

pub fn block_on<F: Future>(f: F) -> Result<F::Output> {
    // let task = Task
    // let b = Box::<dyn Future>::pin(f);
    todo!()
}

static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

thread_local! {
    static CURRENT: Option<NonNull<Executor>> = const { None };
}

pub struct Executor(NonNull<Inner>);
struct Inner {
    // pinned_tasks: xaio_core::collection::SList<Task>,
    epoch: Instant,
    now: AtomicU64,
}

impl Inner {
    const LAYOUT: std::alloc::Layout = unsafe {
        std::alloc::Layout::from_size_align_unchecked(
            std::mem::size_of::<Inner>(),
            std::mem::align_of::<Inner>(),
        )
    };

    pub fn new() -> Result<NonNull<Self>> {
        let ptr = unsafe { std::alloc::alloc(Self::LAYOUT) } as *mut Self;
        if !ptr.is_null() {
            let epoch = *EPOCH;
            unsafe {
                ptr.write(Self {
                    // pinned_tasks: SList::new(),
                    epoch,
                    now: AtomicU64::new(epoch.elapsed().as_millis() as u64),
                })
            };
            Ok(unsafe { NonNull::new_unchecked(ptr) })
        } else {
            Err(Error::from(ErrorKind::OutOfMemory))
        }
    }
}
impl Executor {
    pub fn new() -> Result<Self> {
        Inner::new().map(|inner| Self(inner))
    }

    fn deref(&self) -> &Inner {
        unsafe { self.0.as_ref() }
    }
    fn now(&self) -> u64 {
        self.deref().now.load(Ordering::Relaxed)
    }

    fn update_now(&self) -> u64 {
        let now = self.deref().epoch.elapsed().as_millis() as u64;
        self.deref().now.store(now, Ordering::Relaxed);
        now
    }
}
