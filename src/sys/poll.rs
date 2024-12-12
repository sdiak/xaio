use rustc_hash::FxHashMap;

use crate::selector::rawpoll::{sys_poll, PollFD, POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};
use crate::RawSocketFd;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

pub struct Poll {
    inner: Arc<Inner>,
}
struct Inner {
    fds: Mutex<Registration>,
}

impl Poll {}
// struct Inner {
//     poll_fds: Vec<rawpoll::PollFD>,
//     tokens: Vec<usize>,
//     fd_to_index: HashMap<RawSocketFd, u32>,
//     len: u32,
// }

#[derive(Debug, Clone)]
struct Registration {
    /// sys_poll argument
    fds: Vec<PollFD>,
    /// maps an fd to its index in fds and it's associated token
    data: FxHashMap<RawSocketFd, FdEntry>,
}

#[derive(Debug, Copy, Clone)]
struct FdEntry {
    index_in_fds: usize,
    token: usize,
}
