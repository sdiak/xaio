pub mod executor;
use std::mem::offset_of;

mod boxed_future;
pub use boxed_future::*;

mod task;
pub use task::*;

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
