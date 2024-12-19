use crate::selector::Interest;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PollFD(libc::pollfd);
use crate::RawSocketFd;

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
        Self(libc::pollfd {
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
        Self(libc::pollfd {
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

    let poll_result = unsafe { libc::poll(pfd.as_mut_ptr() as _, pfd.len() as _, timeout as _) };
    if poll_result < 0 {
        let e = std::io::Error::last_os_error();
        Err(e)
    } else {
        Ok(poll_result as usize)
    }
}
