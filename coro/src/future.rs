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
    pub fn wait(self) -> T::Output {
        unsafe { &*self.producer.as_ref().get() }.wait()
        // self.producer.into_inner()
        // unsafe { self.output.assume_init() }
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
