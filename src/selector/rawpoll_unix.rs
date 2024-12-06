pub use libc::pollfd as PollFD;

pub const POLLERR: libc::c_short = libc::POLLERR;
pub const POLLHUP: libc::c_short = libc::POLLHUP;
pub const POLLIN: libc::c_short = libc::POLLIN;
pub const POLLOUT: libc::c_short = libc::POLLOUT;
pub const POLLPRI: libc::c_short = libc::POLLPRI;

pub fn sys_poll(pfd: &mut [PollFD], timeout: libc::c_int) -> std::io::Result<usize> {
    // A bug in kernels < 2.6.37 makes timeouts larger than LONG_MAX / CONFIG_HZ
    // (approx. 30 minutes with CONFIG_HZ=1200) effectively infinite on 32 bits
    // architectures. The magic number is the same constant used by libuv.
    #[cfg(target_pointer_width = "32")]
    let timeout = std::cmp::min(1789569 as libc::c_int, timeout);

    let poll_result = unsafe { libc::poll(pfd.as_mut_ptr(), pfd.len() as _, timeout as _) };
    if poll_result < 0 {
        let e = std::io::Error::last_os_error();
        Err(e.into())
    } else {
        Ok(poll_result as usize)
    }
}
