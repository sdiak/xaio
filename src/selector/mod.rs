use std::time::Duration;

mod poll;
pub use poll::*;
mod event;
pub use event::*;
mod interest;
use crate::RawSocketFd;
pub use interest::*;

pub trait SelectorImpl {
    fn register(
        &mut self,
        fd: RawSocketFd,
        token: usize,
        interests: Interest,
    ) -> std::io::Result<()>;
    fn reregister(
        &mut self,
        fd: RawSocketFd,
        token: usize,
        interests: Interest,
    ) -> std::io::Result<()>;
    fn unregister(&mut self, fd: RawSocketFd) -> std::io::Result<()>;
    fn select(&self, events: &mut [Event], timeout: Option<Duration>) -> std::io::Result<usize>;
}

pub struct Selector {}
