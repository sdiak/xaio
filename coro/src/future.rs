use crate::ptr::Ptr;
use crate::task::{Task, TaskInner};
use crate::{PhantomUnsend, PhantomUnsync};
use std::borrow::Borrow;
use std::io::Result;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};
use std::time::Duration;
use std::{cell::UnsafeCell, mem::MaybeUninit, ptr::NonNull, rc::Rc, sync::atomic::AtomicUsize};

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
    Cancelling = 1,
    Ready = 2,
    Paniced = 3,
}

pub trait FutureListener: Send {
    fn notify(&self, token: usize);
}
pub(crate) struct FutureInner<T: Send, L: FutureListener> {
    state: AtomicU8,
    // link: crate::collection::SLink,
    listener: L,
    value: UnsafeCell<MaybeUninit<T>>,
}
// impl<T: Send, L: FutureListener> crate::collection::SListNode for FutureInner<T, L> {
//     const OFFSET_OF_LINK: usize = std::mem::offset_of!(FutureInner<T, L>, link);
// }
impl<T: Send, L: FutureListener> FutureInner<T, L> {
    pub(crate) fn state(&self, order: Ordering) -> State {
        unsafe { std::mem::transmute::<u8, State>(self.state.load(order)) }
    }

    pub(crate) fn resolve(thiz: Ptr<Self>, value: T) {
        assert!(thiz.state(Ordering::Relaxed) == State::Pending);
        // Perform the write
        unsafe { (&mut *thiz.value.get()).as_mut_ptr().write(value) };
        // Commit the write
        match thiz.state.compare_exchange(
            State::Pending as u8,
            State::Ready as u8,
            Ordering::Release,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                thiz.listener.notify(unsafe { thiz.as_ptr() } as usize);
                // The listener will drop `thiz`
                std::mem::forget(thiz);
            }
            Err(c) => {
                assert!(c == State::Cancelling as u8);
                // Cancelled, drop `value` and `thiz`
                unsafe {
                    (&mut *thiz.value.get()).assume_init_drop();
                }
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
                State::Cancelling => {
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
