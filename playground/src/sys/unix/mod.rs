#[cfg_attr(
    any(target_os = "linux", target_os = "freebsd"),
    path = "event_eventfd.rs"
)]
#[cfg_attr(
    not(any(target_os = "linux", target_os = "freebsd")),
    path = "event_pipe.rs"
)]
mod event;
pub use event::*;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod eventfd;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod epoll;
        pub use epoll::*;
        mod iouring;
        pub use iouring::*;
        mod driver_linux;
        pub use driver_linux::*;
    }
}

mod stat;
pub use stat::*;

#[inline(always)]
pub fn last_os_error() -> i32 {
    unsafe { (*errno_location()) as i32 }
}

fn dup(fd: libc::c_int) -> std::io::Result<libc::c_int> {
    if fd < 0 {
        Ok(fd)
    } else {
        let fd: i32 = unsafe { libc::dup(fd) };
        if fd >= 0 {
            Ok(fd)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
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
