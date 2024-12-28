mod ptr;
pub use ptr::Ptr;
mod completion_port;
pub use completion_port::*;

pub mod driver;

pub type PhantomUnsync = std::marker::PhantomData<std::cell::Cell<()>>;
pub type PhantomUnsend = std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>;
