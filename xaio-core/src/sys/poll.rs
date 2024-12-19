use std::io::{Error, ErrorKind, Result};

use super::RawSocket;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PollFd(libc::pollfd);

cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        pub const MAX_POLL_FDS: usize = libc::nfds_t::MAX as _;
    } else if #[cfg(target_family = "windows")] {
        pub const MAX_POLL_FDS: usize = libc::ulong::MAX as _;
    }
}

bitflags::bitflags! {
    /// Represents a set of input and output flags for poll.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Event: i16 {
        /// Readable interests or event.
        const IN = libc::POLLIN;
        /// Writable interests or event.
        const OUT = libc::POLLOUT;
        /// Priority interests or event.
        const PRI = libc::POLLPRI;

        /// Error event.
        const ERR = libc::POLLERR;
        /// Hang-up event (peer closed its end of the channel).
        const HUP = libc::POLLHUP;
    }
}

impl PollFd {
    #[inline]
    pub fn new(fd: RawSocket, events: Event) -> Self {
        Self(libc::pollfd {
            fd: fd,
            events: events.bits(),
            revents: 0 as _,
        })
    }

    #[inline(always)]
    pub fn fd(&self) -> RawSocket {
        self.0.fd
    }

    #[inline(always)]
    pub fn events(&self) -> Event {
        Event::from_bits_truncate(self.0.events)
    }

    #[inline(always)]
    pub fn revents(&self) -> Event {
        Event::from_bits_truncate(self.0.revents)
    }

    #[inline(always)]
    pub fn set_events(&mut self, events: Event) {
        self.0.events = events.bits();
    }

    #[inline(always)]
    pub fn enable(&mut self, fd: RawSocket, interests: Event) {
        self.0.fd = fd;
        self.0.events = interests.bits();
        self.0.revents = 0;
    }

    #[inline(always)]
    pub fn disable(&mut self) {
        self.0.fd = super::INVALID_RAW_SOCKET;
    }

    #[inline(always)]
    pub fn is_disabled(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(target_family = "unix")] {
                self.0.fd < 0
            } else if #[cfg(target_family = "windows")] {
                self.0.fd == super::INVALID_RAW_SOCKET
            }
        }
    }
}

impl Default for PollFd {
    #[inline(always)]
    fn default() -> Self {
        Self(libc::pollfd {
            fd: super::INVALID_RAW_SOCKET,
            events: 0,
            revents: 0,
        })
    }
}

impl std::fmt::Debug for PollFd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollFd")
            .field("fd", &self.0.fd)
            .field("events", &self.events())
            .field("revents", &self.revents())
            .finish()
    }
}

pub fn poll(pfd: &mut [PollFd], timeout: libc::c_int) -> Result<usize> {
    // A bug in kernels < 2.6.37 makes timeouts larger than LONG_MAX / CONFIG_HZ
    // (approx. 30 minutes with CONFIG_HZ=1200) effectively infinite on 32 bits
    // architectures. The magic number is the same constant used by libuv.
    #[cfg(all(target_os = "linux", target_pointer_width = "32"))]
    let timeout = std::cmp::min(1789569 as libc::c_int, timeout);

    if pfd.len() > MAX_POLL_FDS {
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    cfg_if::cfg_if! {
        if #[cfg(target_family = "unix")] {
            let poll_result = unsafe { libc::poll(pfd.as_mut_ptr() as _, pfd.len() as _, timeout as _) };
            if poll_result < 0 {
                let e = Error::last_os_error();
                Err(e)
            } else {
                Ok(poll_result as usize)
            }
        } else if #[cfg(target_family = "windows")] {
            let poll_result = unsafe { WSAPoll(pfd.as_mut_ptr() as _, pfd.len() as _, timeout as _) };
            if poll_result == SOCKET_ERROR {
                Err(Error::last_os_error())
            } else {
                Ok(poll_result as usize)
            }
        }
    }
}
