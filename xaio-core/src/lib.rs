use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

pub mod completion_queue;
mod io_req;
pub mod io_req_fifo;
pub mod io_req_lifo;
pub mod sys;
pub use io_req::*;

pub mod collection;

pub type PhantomUnsync = std::marker::PhantomData<std::cell::Cell<()>>;
pub type PhantomUnsend = std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>;

fn catch_enomem<C, T>(constructor: C) -> std::io::Result<T>
where
    C: FnOnce() -> T + std::panic::UnwindSafe,
{
    std::panic::catch_unwind(constructor)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}

#[macro_export]
macro_rules! pin_mut {
    ($var:ident) => {
        let mut $var = $var;
        #[allow(unused_mut)]
        let mut $var = unsafe { Pin::new_unchecked(&mut $var) };
    };
}

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
