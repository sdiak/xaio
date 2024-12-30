mod task_box;
pub use task_box::*;

use crate::future::State;

#[derive(Debug)]
pub struct Context {
    state: std::sync::atomic::AtomicU8,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self {
            state: std::sync::atomic::AtomicU8::new(State::Pending as u8),
        }
    }
    pub(crate) fn state(&self, order: std::sync::atomic::Ordering) -> State {
        unsafe { std::mem::transmute::<u8, State>(self.state.load(order)) }
    }
    pub fn is_cancelled(&self) -> bool {
        self.state(std::sync::atomic::Ordering::Relaxed) == State::Cancelled
    }
}
pub trait Task {
    type Output;
    fn resume(self: &mut Self, cx: &mut Context) -> Option<Self::Output>;
}

pub(crate) struct FnTask<F, O>
where
    F: FnOnce() -> O,
{
    f: Option<F>,
}
impl<F, O> FnTask<F, O>
where
    F: FnOnce() -> O,
{
    pub(crate) fn new(f: F) -> Self {
        Self { f: Some(f) }
    }
}

impl<F, O> Task for FnTask<F, O>
where
    F: FnOnce() -> O,
{
    type Output = O;

    fn resume(self: &mut Self, _cx: &mut Context) -> Option<Self::Output> {
        if let Some(f) = self.f.take() {
            Some((f)())
        } else {
            unreachable!("Resume can not be called on a terminated task")
        }
    }
}
