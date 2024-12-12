use std::fmt::Write;

use windows_sys::Win32::Networking::WinSock::{WSAPoll, WSAPOLLFD};
pub use windows_sys::Win32::Networking::WinSock::{POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};

use crate::RawSocketFd;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PollFD(WSAPOLLFD);

impl PollFD {
    pub fn new(fd: RawSocketFd, interests: u16) -> Self {
        Self(WSAPOLLFD {
            fd: fd.inner as _,
            events: interests as _,
            revents: 0 as _,
        })
    }
    pub fn fd(&self) -> RawSocketFd {
        RawSocketFd::new(self.0.fd as _)
    }
    pub fn interests(&self) -> u16 {
        self.0.events as _
    }
    pub fn events(&self) -> u16 {
        self.0.revents as _
    }
    pub fn set_interests(&mut self, interests: u16) {
        self.0.events = interests as _;
    }
    pub fn disable(&mut self) {
        self.0.fd = RawSocketFd::invalid().inner as _;
    }
    pub fn is_disabled(&self) -> bool {
        !RawSocketFd::new(self.0.fd as _).is_valid()
    }
    pub fn enable(&mut self, fd: RawSocketFd, interests: u16) {
        self.0.fd = fd.inner as _;
        self.0.events = interests as _;
        self.0.revents = 0 as _;
    }
}
impl Default for PollFD {
    fn default() -> Self {
        Self(WSAPOLLFD {
            fd: RawSocketFd::invalid().inner as _,
            events: 0 as _,
            revents: 0 as _,
        })
    }
}

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
