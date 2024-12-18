use std::{
    cell::UnsafeCell,
    future::Future,
    io::Result,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{atomic::AtomicBool, Arc, Condvar, Mutex, MutexGuard}, task::{Context, RawWaker, RawWakerVTable, Waker}, thread::Thread,
};

pub type LocalFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
pub type SharedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

struct Task<'a, T> {
    future:  SharedFuture<'a, T>,
}
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

struct CondWaker {
    inner: Arc<Thread>
}
impl CondWaker {
    pub fn new() -> Result<CondWaker> {
        crate::catch_enomem(|| CondWaker{ inner: Arc::new(std::thread::current())})
    }
    pub fn as_waker(&self) -> Waker {
        let arc = self.inner.clone();
        let raw_ptr = Arc::into_raw(arc) as *const ();
        println!("CondWaker::as_waker({:?})", raw_ptr);
        let raw = RawWaker::new(raw_ptr, &CondWaker::VTABLE);;
        unsafe { Waker::from_raw(raw) }
    }
    fn clone(raw_ptr: *const ()) -> RawWaker {
        println!("CondWaker::clone({:?})", raw_ptr);
        let arc = unsafe { Arc::from_raw(raw_ptr as *const Thread) };
        std::mem::forget(arc.clone());
        std::mem::forget(arc);
        RawWaker::new(raw_ptr, &CondWaker::VTABLE)
    }
    fn wake(raw_ptr: *const ()) {
        println!("CondWaker::wake({:?})", raw_ptr);
        CondWaker::wake_by_ref(raw_ptr);
        CondWaker::drop(raw_ptr);
    }
    fn wake_by_ref(raw_ptr: *const ()) {
        println!("CondWaker::wake_by_ref({:?})", raw_ptr);
        let arc = unsafe { Arc::from_raw(raw_ptr as *const Thread) };
        arc.unpark();
        // let mut lock = arc.0.lock().expect("Unrecoverable error");
        // println!("CondWaker::wake_by_ref({:?}): lock: {:?}", raw_ptr, *lock);
        // if *lock {
        //     *lock = false;
        //     arc.1.notify_one();
        //     println!("CondWaker::wake_by_ref({:?}): signaled", raw_ptr);
        // }
        // drop(lock);
        std::mem::forget(arc);
    }
    fn drop(raw_ptr: *const ()) {
        println!("CondWaker::drop({:?})", raw_ptr);
        let _ = unsafe { Arc::from_raw(raw_ptr as *const Thread) };
    }
    
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        CondWaker::clone,
        CondWaker::wake,
        CondWaker::wake_by_ref,
        CondWaker::drop,
    );
}

pub struct Scheduler(Arc<SchedulerInner>);
impl Scheduler {
    pub fn run<O>(task: impl Future<Output = O> + Send + 'static) -> Result<O>
    where
        O: Send,
    {
        let future = unsafe { SharedFuture::new_unchecked(Box::new(task)) };
        // let thread_self = std::thread::current();
        let mut task = Task { future };
        // let waker = unsafe { Waker::from_raw(NOOP) };
        let cnd = CondWaker::new()?;
        let waker = cnd.as_waker();
        let mut cx = Context::from_waker(&waker);
        loop {
            match task.future.as_mut().poll(&mut cx) {
                std::task::Poll::Ready(o) => {return Ok(o);}
                std::task::Poll::Pending => {
                    std::thread::park();
                },
            }
        }
    }
    // pub fn spawn_local<O>(task: impl Future<Output = O> + 'static) -> Result<O> {
    //     let future = unsafe { LocalFuture::new_unchecked(Box::new(task)) };
    //     let task = Task { future };
    //     Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    // }
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
