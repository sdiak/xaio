pub type RawSocket = std::os::fd::RawFd;
pub type RawFd = std::os::fd::RawFd;
pub const INVALID_RAW_SOCKET: RawSocket = -1;
pub const INVALID_RAW_FD: RawFd = -1;

pub mod ioutils;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod eventfd;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod epoll;
        mod iouring;
    }
}
