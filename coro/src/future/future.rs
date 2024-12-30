use crate::ptr::Ptr;

use super::{FutureInner, Promise};

#[derive(Debug)]
pub struct Future2<'a, T: Send>(Ptr<FutureInner<'a, T>>);

// impl<'a, T: Send> Drop for Future2<'a, T> {
//     fn drop(&mut self) {
//         let owned = unsafe { Ptr::from_raw_owned_unchecked(self.0.as_mut_ptr()) };
//         FutureInner::cancel(owned)
//     }
// }

impl<'a, T: Send> Future2<'a, T> {
    pub fn try_pending() -> Option<(Self, Promise<'a, T>)> {
        Ptr::try_new(FutureInner::<'a, T>::pending()).map(|owned| {
            let raw = unsafe { owned.into_raw_unchecked() }; // Promise owns the memory initialy
            unsafe {
                (
                    Self(Ptr::from_raw_unchecked(raw)),
                    Promise::new(Ptr::from_raw_owned_unchecked(raw)),
                )
            }
        })
    }
    pub fn pending() -> (Self, Promise<'a, T>) {
        Self::try_pending().expect("Out of memory")
    }

    pub fn try_ready(value: T) -> Option<Self> {
        Ptr::try_new(FutureInner::<'a, T>::ready(value))
            .map(|owned| unsafe { Self(Ptr::from_raw_unchecked(owned.into_raw_unchecked())) })
    }
    pub fn ready(value: T) -> Self {
        Self::try_ready(value).expect("Out of memory")
    }

    pub fn cancel(mut self) {
        let owned = unsafe { Ptr::from_raw_owned_unchecked(self.0.as_mut_ptr()) };
        FutureInner::cancel(owned)
    }
    pub fn wait(mut self) -> T {
        let owned = unsafe { Ptr::from_raw_owned_unchecked(self.0.as_mut_ptr()) };
        FutureInner::wait(owned)
    }
}
