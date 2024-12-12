use std::time::Duration;

mod poll;
pub use poll::*;
mod event;
pub use event::*;
mod interest;
use crate::RawSocketFd;
pub use interest::*;

pub trait SelectorImpl {
    fn register(&self, fd: RawSocketFd, token: usize, interests: Interest) -> std::io::Result<()>;
    fn reregister(&self, fd: RawSocketFd, token: usize, interests: Interest)
        -> std::io::Result<()>;
    fn unregister(&self, fd: RawSocketFd) -> std::io::Result<()>;
    fn select(&self, events: &mut Vec<Event>, timeout_ms: i32) -> std::io::Result<usize>;
}

pub struct Selector {}
