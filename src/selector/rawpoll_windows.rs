use std::fmt::Write;

use windows_sys::Win32::Networking::WinSock::{WSAPoll, WSAPOLLFD};
pub use windows_sys::Win32::Networking::WinSock::{POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PollFD(WSAPOLLFD);

impl std::fmt::Debug for PollFD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PollFD(TODO:)")
    }
}

pub fn sys_poll(pfd: &mut [PollFD], timeout: libc::c_int) -> std::io::Result<usize> {
    let poll_result = unsafe { WSAPoll(pfd.as_mut_ptr() as _, pfd.len() as _, timeout as _) };
    if poll_result < 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(poll_result as usize)
    }
}
