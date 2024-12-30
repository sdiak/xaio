pub(crate) mod collection;
pub(crate) mod sync;
pub(crate) mod sys;

pub mod future;
pub mod ptr;
pub mod scheduler;
pub mod scope;
pub mod task;

pub type PhantomUnsync = std::marker::PhantomData<std::cell::Cell<()>>;
pub type PhantomUnsend = std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>;

/// A Future has to references : the producer (mutable) and the consumer (immutable)
/// It starts pending until the producer resolves it.
/// The consumer is responsible to drop it unless if it dropped it
struct SharedFuture {}

fn catch_enomem<C, T>(constructor: C) -> std::io::Result<T>
where
    C: FnOnce() -> T + std::panic::UnwindSafe,
{
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(constructor))
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}
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

pub(crate) fn die(message: &str) -> ! {
    log::error!("{}, aborting.", message);
    eprintln!("{}, aborting.", message);
    std::process::abort();
}
