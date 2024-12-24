pub type RawSd = std::os::fd::RawFd;
pub type RawFd = std::os::fd::RawFd;
pub const INVALID_RAW_SD: RawSd = -1;
pub const INVALID_RAW_FD: RawFd = -1;

#[inline(always)]
pub fn raw_sd_is_valid(sd: RawSd) -> bool {
    sd >= 0
}
#[inline(always)]
pub fn raw_fd_is_valid(sd: RawFd) -> bool {
    sd >= 0
}

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
