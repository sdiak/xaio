use listener::{FutureListener, FutureListenerErased};

use crate::ptr::Ptr;
use crate::task::{Task, TaskInner};
use crate::{PhantomUnsend, PhantomUnsync};
mod listener;
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

pub(crate) struct FutureInner<'a, T: Send> {
    listener_and_state: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
    _lifetime: PhantomData<&'a FutureListenerErased>,
}

// impl<T: Send, L: FutureListener> crate::collection::SListNode for FutureInner<T, L> {
//     const OFFSET_OF_LINK: usize = std::mem::offset_of!(FutureInner<T, L>, link);
// }
impl<'a, T: Send> FutureInner<'a, T> {
    pub fn ready(val: T) -> Self {
        Self {
            listener_and_state: AtomicUsize::new(State::Ready as u8 as _),
            value: UnsafeCell::new(MaybeUninit::new(val)),
            _lifetime: PhantomData {},
        }
    }
    pub(crate) fn state(&self, order: Ordering) -> State {
        unsafe { std::mem::transmute::<u8, State>((self.listener_and_state.load(order) & 3) as u8) }
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
                // Notify listener
                let listener =
                    unsafe { &*((listener_and_state & !3usize) as *const FutureListenerErased) };
                listener.notify(unsafe { thiz.as_ptr() } as usize);
                // The listener will drop `thiz`
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