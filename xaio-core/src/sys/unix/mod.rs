pub type RawSd = std::os::fd::RawFd;
pub type RawFd = std::os::fd::RawFd;
pub const INVALID_RAW_SD: RawSd = -1;
pub const INVALID_RAW_FD: RawFd = -1;

pub mod ioutils;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod eventfd;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub mod epoll;
        pub mod iouring;
    }
}

#[cfg_attr(target_os = "linux", path = "io_driver_linux.rs")]
pub mod io_driver;

#[cfg_attr(target_os = "linux", path = "epoll_selector.rs")]
pub mod selector;

pub mod statx_impl;
