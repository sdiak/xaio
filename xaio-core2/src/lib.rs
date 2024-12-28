mod ptr;
pub use ptr::Ptr;
mod completion_port;
pub use completion_port::*;
mod request;
pub use request::*;

pub mod collection;
pub mod driver;

mod sys;

pub type PhantomUnsync = std::marker::PhantomData<std::cell::Cell<()>>;
pub type PhantomUnsend = std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>;

pub trait Unpark {
    fn unpark(&self);
}
// #[derive(Clone)]
pub struct Unparker {
    target: Box<dyn Unpark>,
}
impl Unparker {
    pub fn new<U: Unpark + std::panic::UnwindSafe + Send + Sync + Clone + 'static>(
        target: U,
    ) -> Self {
        Self {
            target: Box::new(target),
        }
    }
    pub fn unpark(&self) {
        self.target.unpark();
    }
}

#[derive(Clone)]
pub struct ThreadUnpark(std::thread::Thread);
impl Unpark for ThreadUnpark {
    fn unpark(&self) {
        self.0.unpark();
        // println!("Unpark");
    }
}
