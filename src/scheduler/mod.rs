use std::{
    cell::UnsafeCell,
    future::Future,
    io::{Error, ErrorKind, Result},
    pin::Pin,
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, RawWaker, RawWakerVTable, Waker},
    thread::Thread,
};

pub mod executor;

macro_rules! pin_mut {
    ($var:ident) => {
        let mut $var = $var;
        #[allow(unused_mut)]
        let mut $var = unsafe { Pin::new_unchecked(&mut $var) };
    };
}

thread_local! {
    static CURRENT_TASK: Option<NonNull<Task>> = const { None };
}

pub type LocalFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
pub type SharedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

struct Task {}
// struct Task<'a, T> {
//     future: SharedFuture<'a, T>,
// }
pub struct Executor {
    should_run: AtomicBool,
}
impl Executor {}

const NOOP: RawWaker = {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        // Cloning just returns a new no-op raw waker
        |_| NOOP,
        // `wake` does nothing
        |_| {},
        // `wake_by_ref` does nothing
        |_| {},
        // Dropping does nothing as we don't allocate anything
        |_| {},
    );
    RawWaker::new(std::ptr::null(), &VTABLE)
};

struct ThreadWaker {
    // TODO: uses an atomic flag to avoid other function using park/unpark
    // we should use a custom ThreadParker instead
    inner: Arc<(Thread, AtomicBool)>, // TODO: Thread::{into_raw, from_raw} https://github.com/rust-lang/rust/issues/97523
}
impl ThreadWaker {
    pub fn new() -> Result<ThreadWaker> {
        crate::catch_enomem(|| ThreadWaker {
            inner: Arc::new((std::thread::current(), AtomicBool::new(false))),
        })
    }
    pub fn as_waker(&self) -> Waker {
        let arc = self.inner.clone();
        let raw_ptr = Arc::into_raw(arc) as *const ();
        println!("ThreadWaker::as_waker({:?})", raw_ptr);
        let raw = RawWaker::new(raw_ptr, &ThreadWaker::VTABLE);
        unsafe { Waker::from_raw(raw) }
    }
    fn clone(raw_ptr: *const ()) -> RawWaker {
        println!("ThreadWaker::clone({:?})", raw_ptr);
        let arc = unsafe { Arc::from_raw(raw_ptr as *const (Thread, AtomicBool)) };
        std::mem::forget(arc.clone());
        std::mem::forget(arc);
        RawWaker::new(raw_ptr, &ThreadWaker::VTABLE)
    }
    fn wake(raw_ptr: *const ()) {
        println!("ThreadWaker::wake({:?})", raw_ptr);
        ThreadWaker::wake_by_ref(raw_ptr);
        ThreadWaker::drop(raw_ptr);
    }
    fn wake_by_ref(raw_ptr: *const ()) {
        println!("ThreadWaker::wake_by_ref({:?})", raw_ptr);
        let arc = unsafe { Arc::from_raw(raw_ptr as *const (Thread, AtomicBool)) };
        if !arc.1.swap(true, Ordering::Release) {
            arc.0.unpark();
        }
        std::mem::forget(arc);
    }
    fn wait(&self) {
        while !self.inner.1.swap(false, Ordering::Acquire) {
            std::thread::park();
        }
    }
    fn drop(raw_ptr: *const ()) {
        println!("ThreadWaker::drop({:?})", raw_ptr);
        let _ = unsafe { Arc::from_raw(raw_ptr as *const (Thread, AtomicBool)) };
    }

    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        ThreadWaker::clone,
        ThreadWaker::wake,
        ThreadWaker::wake_by_ref,
        ThreadWaker::drop,
    );
}

pub struct Scheduler(Arc<SchedulerInner>);
impl Scheduler {}

pub fn block_on<F: Future>(task: F) -> Result<F::Output> {
    pin_mut!(task);
    // TODO: create a forein task and set CURRENT_TASK
    let thrd = ThreadWaker::new()?;
    let waker = thrd.as_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        if let std::task::Poll::Ready(out) = task.as_mut().poll(&mut cx) {
            return Ok(out);
        }
        thrd.wait();
    }
}
// impl Deref for Scheduler {
//     type Target = Shared;
//     fn deref(&self) -> &Self::Target {
//         unsafe { &*self.0 .0.get() }
//     }
// }
// impl DerefMut for Scheduler {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         unsafe { &mut *self.0 .0.get() }
//     }
// }
// unsafe impl Send for Scheduler {}
// unsafe impl Sync for Scheduler {}

struct Shared {
    executors: Vec<Executor>,
}
struct SchedulerInner(UnsafeCell<Shared>);
