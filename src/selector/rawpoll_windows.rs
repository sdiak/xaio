use  windows_sys::Win32::Networking::WinSock::WSAPoll;
pub use  windows_sys::Win32::Networking::WinSock::{POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI, WSAPOLLFD as PollFD};


pub fn sys_poll(pfd: &mut [PollFD], timeout: libc::c_int) -> std::io::Result<usize> {
    let poll_result = unsafe { WSAPoll(pfd.as_mut_ptr(), pfd.len() as _, timeout as _) };
    if poll_result < 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(poll_result as usize)
    }
}
