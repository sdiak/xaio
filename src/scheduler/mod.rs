use std::{
    cell::UnsafeCell,
    future::Future,
    io::{Error, ErrorKind, Result},
    pin::Pin,
    ptr::NonNull,
    sync::{atomic::AtomicBool, Arc},
    task::{Context, RawWaker, RawWakerVTable, Waker},
    thread::Thread,
};

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
    inner: Arc<Thread>, // TODO: Thread::{into_raw, from_raw} https://github.com/rust-lang/rust/issues/97523
}
impl ThreadWaker {
    pub fn new() -> Result<ThreadWaker> {
        crate::catch_enomem(|| ThreadWaker {
            inner: Arc::new(std::thread::current()),
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
        let arc = unsafe { Arc::from_raw(raw_ptr as *const Thread) };
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
        let arc = unsafe { Arc::from_raw(raw_ptr as *const Thread) };
        arc.unpark();
        // let mut lock = arc.0.lock().expect("Unrecoverable error");
        // println!("ThreadWaker::wake_by_ref({:?}): lock: {:?}", raw_ptr, *lock);
        // if *lock {
        //     *lock = false;
        //     arc.1.notify_one();
        //     println!("ThreadWaker::wake_by_ref({:?}): signaled", raw_ptr);
        // }
        // drop(lock);
        std::mem::forget(arc);
    }
    fn drop(raw_ptr: *const ()) {
        println!("ThreadWaker::drop({:?})", raw_ptr);
        let _ = unsafe { Arc::from_raw(raw_ptr as *const Thread) };
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
    let cnd = ThreadWaker::new()?;
    let waker = cnd.as_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match task.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(o) => {
                return Ok(o);
            }
            std::task::Poll::Pending => {
                std::thread::park();
            }
        }
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
