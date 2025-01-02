mod context;
pub(crate) mod tagged_ptr;
use std::{
    borrow::BorrowMut,
    cell::UnsafeCell,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

pub use context::*;

pub(self) mod wait_list;
use wait_list::WaitList;

pub struct ParkingSpot {
    token: AtomicUsize,
    mutex: Mutex<UnsafeCell<WaitList>>,
}

impl ParkingSpot {
    pub fn park(&self, blocking_token: usize) {
        let mut wait_list = self.mutex.lock().expect("Unrecoverable error");
        if self.token.load(Ordering::Acquire) == blocking_token {
            Context::with_current(move |cx| {
                cx.blocking_token = blocking_token;
                (&mut *wait_list)
                    .get_mut()
                    .push_back(unsafe { NonNull::new_unchecked(cx as _) });
                drop(wait_list); // Releases the wait-list lock before parking
                todo!("Park");
            });
        }
    }
    pub fn unpark(&self, max: std::num::NonZero<usize>) -> usize {
        let mut wait_list = self.mutex.lock().expect("Unrecoverable error");
        let current_token = self.token.load(Ordering::Acquire);
        let mut n_removed = 0usize;
        let mut to_unpark = (&mut *wait_list).get_mut().retain(move |cx| {
            // TODO:
            if n_removed >= max.get() {
                return true;
            }
            let result = cx.blocking_token != current_token;
            n_removed += result as usize;
            result
        });
        drop(wait_list); // Releases the wait-list lock before unparking
        while let Some(cx) = to_unpark.pop_front() {
            todo!("unpark");
        }
        n_removed
    }
}
