use crate::ptr::Ptr;
use crate::task::{Task, TaskInner};
use crate::{PhantomUnsend, PhantomUnsync};
use std::borrow::Borrow;
use std::io::Result;
use std::sync::atomic::{AtomicU8, Ordering};
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
