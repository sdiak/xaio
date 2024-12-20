#![allow(dead_code)]

mod buffer;
pub mod capi;
mod details;
mod driver;
mod driver_none;
mod driver_uring;
mod fd_map;
mod file;
mod ready_fifo;
mod ready_list;
mod request;
mod request_list;
mod request_queue;
mod ring;
mod selector;
mod socket;
mod stat;
mod thread_pool;
pub use driver::*;
pub use driver_none::*;
pub use ready_list::*;
pub use request::*;
pub use request_list::*;
pub use ring::*;
pub use socket::RawSocketFd;

pub mod scheduler;


pub mod sys;

pub use socket::*;
mod utils;
pub(crate) use utils::*;

pub type PhantomUnsync = std::marker::PhantomData<std::cell::Cell<()>>;
pub type PhantomUnsend = std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>;

#[cfg(target_os = "linux")]
mod driver_epoll;

#[cfg(target_os = "windows")]
mod driver_iocp_windows;

#[cfg(not(target_os = "windows"))]
pub(crate) unsafe fn saturating_duration_to_timespec(
    duration: &std::time::Duration,
    mem: &mut libc::timespec,
) -> *const libc::timespec {
    if duration.as_secs() > libc::time_t::MAX as u64 {
        mem.tv_sec = libc::time_t::MAX;
        mem.tv_nsec = 999_999_999;
    } else {
        mem.tv_sec = duration.as_secs() as _;
        mem.tv_nsec = duration.subsec_nanos() as _;
    }
    mem as _
}
#[cfg(not(target_os = "windows"))]
pub(crate) unsafe fn saturating_opt_duration_to_timespec(
    duration: Option<std::time::Duration>,
    mem: &mut libc::timespec,
) -> *const libc::timespec {
    match duration {
        Some(duration) => saturating_duration_to_timespec(&duration, mem),
        None => std::ptr::null(),
    }
}
#[inline]
pub(crate) fn saturating_duration_to_ms(duration: &std::time::Duration) -> libc::c_int {
    // At least one ms when duration > 0
    num::clamp(
        duration
            .saturating_add(std::time::Duration::from_nanos(999_999))
            .as_millis(),
        0,
        libc::c_int::MAX as u128,
    ) as libc::c_int
}
pub(crate) fn saturating_opt_duration_to_ms(duration: Option<std::time::Duration>) -> libc::c_int {
    match duration {
        Some(duration) => saturating_duration_to_ms(&duration),
        None => -1,
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // println!("Hello\n");
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

fn catch_enomem<C, T>(constructor: C) -> std::io::Result<T>
where
    C: FnOnce() -> T + std::panic::UnwindSafe,
{
    std::panic::catch_unwind(constructor)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}

mod status;
pub use status::*;

pub type CTaskPoll = extern "C" fn(thiz: &mut CTask) -> status::Status;
#[repr(C)]
#[derive(Debug)]
pub struct CTask {
    poll: CTaskPoll,
    status: std::sync::atomic::AtomicIsize,
}

struct RustTask<T> {
    result: std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'static>>,
}

union TaskFlavor<T> {
    as_rust: std::mem::ManuallyDrop<RustTask<T>>,
    as_c: std::mem::ManuallyDrop<CTask>,
}
struct TaskInner<T> {
    /// The poll callback
    flavor: TaskFlavor<T>,
    /// The worker address and 1usize when this is a c task flavor
    owner: usize,
}
struct Task {
    // TODO:
}
impl Task {
    fn poll(&mut self) -> bool {
        false
    }
}
