use std::fmt::Write;

use windows_sys::Win32::Networking::WinSock::{WSAPoll, WSAPOLLFD};
pub use windows_sys::Win32::Networking::WinSock::{POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};

use crate::selector::Interest;
use crate::RawSocketFd;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PollFD(WSAPOLLFD);

impl PollFD {
    pub fn interest_to_events(interest: Interest) -> i16 {
        let mut r: i16 = 0;
        if interest.contains(Interest::READABLE) {
            r |= POLLIN;
        }
        if interest.contains(Interest::WRITABLE) {
            r |= POLLOUT;
        }
        if interest.contains(Interest::PRIORITY) {
            r |= POLLPRI;
        }
        r
    }
    pub fn events_to_interest(events: i16) -> Interest {
        let mut r = Interest::from_bits_retain(0u32);
        if (events & POLLIN) != 0 {
            r |= Interest::READABLE;
        }
        if (events & POLLOUT) != 0 {
            r |= Interest::WRITABLE;
        }
        if (events & POLLPRI) != 0 {
            r |= Interest::PRIORITY;
        }
        if (events & POLLERR) != 0 {
            r |= Interest::ERROR;
        }
        if (events & POLLHUP) != 0 {
            r |= Interest::HANG_UP;
        }
        r
    }
    pub fn new(fd: RawSocketFd, interests: i16) -> Self {
        Self(WSAPOLLFD {
            fd: fd.inner as _,
            events: interests,
            revents: 0 as _,
        })
    }
    pub fn fd(&self) -> RawSocketFd {
        RawSocketFd::new(self.0.fd as _)
    }
    pub fn events(&self) -> i16 {
        self.0.events
    }
    pub fn revents(&self) -> i16 {
        self.0.revents
    }
    pub fn set_events(&mut self, interests: i16) {
        self.0.events = interests;
    }
    pub fn disable(&mut self) {
        self.0.fd = RawSocketFd::invalid().inner as _;
    }
    pub fn is_disabled(&self) -> bool {
        !RawSocketFd::new(self.0.fd as _).is_valid()
    }
    pub fn enable(&mut self, fd: RawSocketFd, interests: i16) {
        self.0.fd = fd.inner as _;
        self.0.events = interests;
        self.0.revents = 0;
    }
}
impl Default for PollFD {
    fn default() -> Self {
        Self(WSAPOLLFD {
            fd: RawSocketFd::invalid().inner as _,
            events: 0,
            revents: 0,
        })
    }
}

impl std::fmt::Debug for PollFD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollFd")
            .field("fd", &(self.0.fd as *const libc::c_void))
            .field("events", &format!("{:#06x}", self.0.events))
            .field("revents", &format!("{:#06x}", self.0.revents))
            .finish()
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
