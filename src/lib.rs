mod capi;
mod driver;
mod driver_none;
mod fd_map;
mod selector;
mod socket;
mod sub;
mod sub_list;
pub use driver::*;
pub use driver_none::*;
pub use socket::RawSocketFd;
pub use sub::*;
pub use sub_list::*;

#[cfg(target_os = "linux")]
mod driver_epoll;

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
