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
    } else if #[cfg(has_kqueue)] {
        mod selector_kqueue;
    }
}

#[cfg_attr(target_os = "linux", path = "io_driver_linux.rs")]
pub mod io_driver;

#[cfg_attr(target_os = "linux", path = "epoll_selector.rs")]
pub mod selector;

pub mod statx_impl;

// FIXME:
// mod selector_kqueue;

#[inline(always)]
pub fn last_os_error() -> i32 {
    unsafe { (*errno_location()) as i32 }
}

extern "C" {
    #[cfg_attr(
        any(
            target_os = "macos",
            target_os = "ios",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "visionos",
            target_os = "freebsd"
        ),
        link_name = "__error"
    )]
    #[cfg_attr(
        any(
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "android",
            target_os = "espidf",
            target_env = "newlib"
        ),
        link_name = "__errno"
    )]
    #[cfg_attr(
        any(target_os = "solaris", target_os = "illumos"),
        link_name = "___errno"
    )]
    #[cfg_attr(target_os = "haiku", link_name = "_errnop")]
    #[cfg_attr(
        any(
            target_os = "linux",
            target_os = "hurd",
            target_os = "redox",
            target_os = "dragonfly",
            target_os = "emscripten",
        ),
        link_name = "__errno_location"
    )]
    #[cfg_attr(target_os = "aix", link_name = "_Errno")]
    #[cfg_attr(target_os = "nto", link_name = "__get_errno_ptr")]
    fn errno_location() -> *mut libc::c_int;
}
