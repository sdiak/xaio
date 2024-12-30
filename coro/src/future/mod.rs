use listener::{FutureListener, FutureListenerErased};

use crate::ptr::Ptr;
use crate::task::{Task, TaskInner};
use crate::{PhantomUnsend, PhantomUnsync};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::UnsafeCell, mem::MaybeUninit};

mod future;
mod listener;
pub use future::*;
mod promise;
pub use promise::*;

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
impl<'a, T: Send> Debug for FutureInner<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FutureInner")
            .field("state", &self.state(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

const TAG_HAS_VALUE: usize = 4;
const TAG_HAS_CONSUMER: usize = 8;
const TAG_HAS_PRODUCER: usize = 16;
// impl<T: Send, L: FutureListener> crate::collection::SListNode for FutureInner<T, L> {
//     const OFFSET_OF_LINK: usize = std::mem::offset_of!(FutureInner<T, L>, link);
// }
impl<'a, T: Send> FutureInner<'a, T> {
    pub(crate) fn pending_with_listener(listener: &'a FutureListenerErased) -> Self {
        let listener = listener as *const FutureListenerErased as usize;
        debug_assert!((listener & 31) == 0, "Alignement allows tags");
        Self {
            listener_and_state: AtomicUsize::new(
                (State::Pending as u8 as usize) | TAG_HAS_CONSUMER | TAG_HAS_PRODUCER | listener,
            ),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _lifetime: PhantomData {},
        }
    }
    pub(crate) fn pending() -> Self {
        Self {
            listener_and_state: AtomicUsize::new(
                State::Pending as usize | TAG_HAS_CONSUMER | TAG_HAS_PRODUCER,
            ),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _lifetime: PhantomData {},
        }
    }
    pub(crate) fn ready(val: T) -> Self {
        Self {
            listener_and_state: AtomicUsize::new(
                State::Ready as usize | TAG_HAS_CONSUMER | TAG_HAS_VALUE,
            ),
            value: UnsafeCell::new(MaybeUninit::new(val)),
            _lifetime: PhantomData {},
        }
    }

    pub(crate) fn set_listener(
        &'a self,
        listener: &'a FutureListenerErased,
    ) -> std::io::Result<bool> {
        let listener = listener as *const FutureListenerErased as usize;
        debug_assert!((listener & 31) == 0, "Alignement allows tags");
        let mut listener_and_state = self.listener_and_state.load(Ordering::Acquire);
        let mut state = unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) };
        loop {
            match state {
                State::Pending => {
                    if (listener_and_state & !31) != 0 {
                        return Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy));
                    }
                    match self.listener_and_state.compare_exchange(
                        listener_and_state,
                        (listener_and_state & 31) | listener,
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
        debug_assert!((listener & 31) == 0, "Alignement allows tags");
        match self.listener_and_state.compare_exchange(
            listener | TAG_HAS_CONSUMER | TAG_HAS_PRODUCER | State::Pending as u8 as usize,
            State::Pending as u8 as usize,
            Ordering::Release,
            Ordering::Acquire,
        ) {
            Ok(_) => return true,
            Err(listener_and_state) => {
                match unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) } {
                    State::Pending if ((listener_and_state & !31usize) != 0) => {
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
        // TODO: flags
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
                crate::die("Future cancelled multiple times");
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

    fn wait(thiz: Ptr<Self>) -> T {
        // TODO: flags
        let mut listener_and_state = thiz.listener_and_state.load(Ordering::Acquire);
        loop {
            let state = unsafe { std::mem::transmute::<u8, State>((listener_and_state & 3) as u8) };
            match state {
                State::Pending => {
                    eprintln!("TODO: get listener reference (current task or current thread)");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    listener_and_state = thiz.listener_and_state.load(Ordering::Acquire);
                }
                State::Cancelled => crate::die("Waiting on a cancelled future"),
                State::Ready => {
                    match thiz.listener_and_state.compare_exchange(
                        listener_and_state,
                        State::Cancelled as _,
                        Ordering::Release,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => {
                            let value = unsafe { (&mut *thiz.value.get()).assume_init_read() };
                            unsafe { thiz.deallocate_without_dropping() };
                            return value;
                        }
                        Err(x) => {
                            listener_and_state = x;
                        }
                    }
                }
                State::Paniced => {
                    // Paniced, drop `thiz`
                    panic!("Future paniced");
                }
            }
        }
    }

    pub(crate) fn resolve(thiz: Ptr<Self>, value: T) {
        // TODO: flags
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use super::*;

    #[test]
    fn test_simple() {
        let f = Future2::ready(42);

        assert_eq!(f.wait(), 42);
        println!("ICI\n");
    }
}
