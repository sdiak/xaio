use crate::ptr::Ptr;

use super::FutureInner;

#[derive(Debug)]
pub struct Promise<'a, T: Send>(Ptr<FutureInner<'a, T>>);

impl<'a, T: Send> Promise<'a, T> {
    pub(crate) fn new(ptr: Ptr<FutureInner<'a, T>>) -> Self {
        Self(ptr)
    }
    pub fn resolve(self, value: T) {
        FutureInner::resolve(self.0, value);
    }
}
