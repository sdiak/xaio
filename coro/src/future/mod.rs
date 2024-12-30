use listener::{FutureListener, FutureListenerErased};

use crate::ptr::Ptr;
use crate::task::{Task, TaskInner};
use crate::{PhantomUnsend, PhantomUnsync};
mod listener;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::UnsafeCell, mem::MaybeUninit};

#[derive(Debug)]
pub struct Future<T: Task> {
    producer: Ptr<UnsafeCell<TaskInner<T>>>,
    _unsend: PhantomUnsend,
    _unsync: PhantomUnsync,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Pending = 0,
    Cancelled = 1,
    Ready = 2,
    Paniced = 3,
}

#[derive(Debug)]
pub struct Future2<'a, T: Send>(Ptr<FutureInner<'a, T>>);

impl<'a, T: Send> Drop for Future2<'a, T> {
    fn drop(&mut self) {
        let owned = unsafe { Ptr::from_raw_owned_unchecked(self.0.as_mut_ptr()) };
        FutureInner::cancel(owned);
    }
}

impl<'a, T: Send> Future2<'a, T> {
    pub fn try_pending() -> Option<(Self, Promise<'a, T>)> {
        Ptr::try_new(FutureInner::<'a, T>::pending()).map(|owned| {
            let raw = unsafe { owned.into_raw_unchecked() }; // Promise owns the memory initialy
            unsafe {
                (
                    Future2(Ptr::from_raw_unchecked(raw)),
                    Promise(Ptr::from_raw_owned_unchecked(raw)),
                )
            }
        })
    }
    // fn wait(self) ->
}

#[derive(Debug)]
pub struct Promise<'a, T: Send>(Ptr<FutureInner<'a, T>>);

impl<'a, T: Send> Promise<'a, T> {
    pub fn resolve(self, value: T) {
        FutureInner::resolve(self.0, value);
    }
}

pub(crate) struct FutureInner<'a, T: Send> {
    listener_and_state: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
    _lifetime: PhantomData<&'a FutureListenerErased>,
}
impl<'a, T: Send> Debug for FutureInner<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FutureInner")
            .field("state", &self.state(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

// impl<T: Send, L: FutureListener> crate::collection::SListNode for FutureInner<T, L> {
//     const OFFSET_OF_LINK: usize = std::mem::offset_of!(FutureInner<T, L>, link);
// }
impl<'a, T: Send> FutureInner<'a, T> {
    pub(crate) fn pending_with_listener(listener: &'a FutureListenerErased) -> Self {
        let listener = listener as *const FutureListenerErased as usize;
        debug_assert!((listener & 3) == 0, "Alignement allows tags");
        Self {
            listener_and_state: AtomicUsize::new((State::Pending as u8 as usize) | listener),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _lifetime: PhantomData {},
        }
    }
    pub(crate) fn pending() -> Self {
        Self {
            listener_and_state: AtomicUsize::new(State::Ready as u8 as _),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _lifetime: PhantomData {},
        }
    }
    pub(crate) fn ready(val: T) -> Self {
        Self {
            listener_and_state: AtomicUsize::new(State::Ready as u8 as _),
            value: UnsafeCell::new(MaybeUninit::new(val)),
            _lifetime: PhantomData {},
        }
    }

    pub(crate) fn set_listener(
        &'a self,
        listener: &'a FutureListenerErased,
    ) -> std::io::Result<bool> {
        let listener = listener as *const FutureListenerErased as usize;
        debug_assert!((listener & 3) == 0, "Alignement allows tags");
        let mut listener_and_state = self.listener_and_state.load(Ordering::Acquire);
        let mut state = unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) };
        loop {
            match state {
                State::Pending => {
                    if (listener_and_state & !3) != 0 {
                        return Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy));
                    }
                    match self.listener_and_state.compare_exchange(
                        listener_and_state,
                        (listener_and_state & 3) | listener,
                        Ordering::Release,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => return Ok(true),
                        Err(x) => {
                            listener_and_state = x;
                            state = unsafe {
                                std::mem::transmute::<u8, State>((listener_and_state & 3) as u8)
                            };
                        }
                    }
                }
                State::Cancelled => return Ok(false),
                State::Ready => return Ok(false),
                State::Paniced => return Ok(false),
            }
        }
    }

    pub(crate) fn rem_listener(&'a self, listener: &'a FutureListenerErased) -> bool {
        let listener = listener as *const FutureListenerErased as usize;
        debug_assert!((listener & 3) == 0, "Alignement allows tags");
        match self.listener_and_state.compare_exchange(
            listener | State::Pending as u8 as usize,
            State::Pending as u8 as usize,
            Ordering::Release,
            Ordering::Acquire,
        ) {
            Ok(_) => return true,
            Err(listener_and_state) => {
                match unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) } {
                    State::Pending if ((listener_and_state & !3usize) != 0) => {
                        crate::die("Another listener is registered")
                    }
                    _ => return false,
                }
            }
        }
    }

    pub(crate) fn state(&self, order: Ordering) -> State {
        unsafe { std::mem::transmute::<u8, State>((self.listener_and_state.load(order) & 3) as u8) }
    }

    pub(crate) fn cancel(thiz: Ptr<Self>) {
        let listener_and_state = thiz
            .listener_and_state
            .swap(State::Cancelled as u8 as usize, Ordering::AcqRel);
        let state = unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) };
        match state {
            State::Pending => {
                // Cancelation registered, producer will drop `thiz`
                std::mem::forget(thiz);
            }
            State::Cancelled => {
                // Already cancelled
                log::warn!("Future cancelled multiple times");
            }
            State::Ready => {
                // Already done, drop `value` and `thiz`
                unsafe {
                    (&mut *thiz.value.get()).assume_init_drop();
                };
            }
            State::Paniced => {
                // Paniced, drop `value` and `thiz`
                unsafe {
                    (&mut *thiz.value.get()).assume_init_drop();
                };
                panic!("Future paniced");
            }
        }
    }

    pub(crate) fn resolve(thiz: Ptr<Self>, value: T) {
        assert!(thiz.state(Ordering::Relaxed) == State::Pending);
        // Perform the write
        unsafe { (&mut *thiz.value.get()).as_mut_ptr().write(value) };
        // Commit the write (make it visible to listener)
        let listener_and_state = thiz
            .listener_and_state
            .swap(State::Ready as u8 as usize, Ordering::AcqRel);
        let state = unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) };
        match state {
            State::Pending => {
                let listener_and_state = listener_and_state & !3usize;
                if listener_and_state != 0 {
                    // Notify listener
                    let listener = unsafe {
                        &*((listener_and_state & !3usize) as *const FutureListenerErased)
                    };
                    listener.notify(unsafe { thiz.as_ptr() } as usize);
                }
                // The consumer will drop `thiz`
                std::mem::forget(thiz);
            }
            State::Cancelled => {
                // Cancelled, drop `value` and `thiz`
                unsafe {
                    (&mut *thiz.value.get()).assume_init_drop();
                }
            }
            _ => {
                unreachable!("Future is already resolved");
            }
        }
    }
}

impl<T: Task> Future<T> {
    pub(crate) fn new(producer: Ptr<UnsafeCell<TaskInner<T>>>) -> Self {
        Self {
            producer,
            _unsend: PhantomUnsend {},
            _unsync: PhantomUnsync {},
        }
    }
    pub fn state(&self, order: Ordering) -> State {
        unsafe { &*self.producer.as_ref().get() }.state(order)
    }

    // pub fn wait_timeout(&self, dur: Duration) -> Option<&Result<T>> {
    //     match self.state(Ordering::Acquire) {
    //         State::Pending => todo!(),
    //         State::Cancelling => todo!(),
    //         State::Ready => Some(unsafe { self.value.assume_init_ref() }),
    //         State::Paniced => panic!("Producer paniced"),
    //     }
    // // }
    pub fn wait(mut self) -> T::Output {
        let inner = unsafe { &mut *self.producer.as_mut().get() };
        loop {
            match self.state(Ordering::Acquire) {
                State::Pending => {
                    std::thread::park();
                }
                State::Cancelled => {
                    std::thread::park();
                }
                State::Ready => return unsafe { inner.take() },
                State::Paniced => panic!("Producer paniced"),
            }
        }
    }
}
/*
#[derive(Debug)]
pub(crate) struct ExecutorInner {}


#[derive(Debug)]
struct Inner<T> {
    // Option<NonNull<>>
    /// The executor that is listening to this future
    owner: &'static ExecutorInner,
    state: AtomicU8,
    value: MaybeUninit<Result<T>>,
}

struct TmpTask {}
// struct Promise<'a, T> {
//     owner: &'a TmpTask,
// }

impl<T> Inner<T> {
    pub fn state(&self, order: Ordering) -> State {
        unsafe { std::mem::transmute::<u8, State>(self.state.load(order)) }
    }

    pub fn wait_timeout(&self, dur: Duration) -> Option<&Result<T>> {
        match self.state(Ordering::Acquire) {
            State::Pending => todo!(),
            State::Cancelling => todo!(),
            State::Ready => Some(unsafe { self.value.assume_init_ref() }),
            State::Paniced => panic!("Producer paniced"),
        }
    }
    pub fn wait(self) -> Result<T> {
        match self.state(Ordering::Acquire) {
            State::Pending => todo!(),
            State::Cancelling => todo!(),
            State::Ready => unsafe { self.value.assume_init() },
            State::Paniced => panic!("Producer paniced"),
        }
    }
}
 */
