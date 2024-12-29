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
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
