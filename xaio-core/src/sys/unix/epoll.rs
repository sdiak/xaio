use super::eventfd::EventFd;
use std::io::ErrorKind;
use std::{
    io::{Error, Result},
    os::fd::{AsRawFd, RawFd},
};

const WAKER_TOKEN: u64 = u64::MAX;

#[derive(Debug)]
pub struct EPoll {
    handle: libc::c_int,
    waker: EventFd,
}

impl AsRawFd for EPoll {
    fn as_raw_fd(&self) -> RawFd {
        self.handle
    }
}

bitflags::bitflags! {
    /// Represents a set of input and output flags for poll.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Event: libc::c_int {
        /// Readable interests or event.
        const IN = libc::EPOLLIN;
        /// Writable interests or event.
        const OUT = libc::EPOLLOUT;
        /// Priority interests or event.
        const PRI = libc::EPOLLPRI;

        /// Error event.
        const ERR = libc::EPOLLERR;
        /// Hang-up event (peer closed its end of the channel).
        const HUP = libc::EPOLLHUP;
        /// Hang-up event (peer closed its end of the channel).
        const RDHUP = libc::EPOLLRDHUP;

        const EXCLUSIVE = libc::EPOLLEXCLUSIVE;

        const EDGE_TRIGGERED = libc::EPOLLET;
    }
}

impl EPoll {
    pub fn invalid() -> Self {
        Self {
            handle: -1,
            waker: EventFd::invalid(),
        }
    }

    pub fn new(close_on_exec: bool) -> Result<Self> {
        let waker = EventFd::new(0, false)?;
        let epfd = unsafe {
            libc::epoll_create1(if close_on_exec {
                libc::EPOLL_CLOEXEC
            } else {
                0
            })
        };
        if epfd >= 0 {
            let epoll = EPoll {
                handle: epfd,
                waker,
            };
            let eventfd = epoll.waker.as_raw_fd();
            epoll.ctl(
                eventfd,
                libc::EPOLL_CTL_ADD,
                Event::IN | Event::EXCLUSIVE,
                WAKER_TOKEN,
            )?;
            Ok(epoll)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn try_clone(&self) -> Result<Self> {
        let waker = self.waker.try_clone()?;
        Ok(Self {
            handle: super::ioutils::dup(self.handle)?,
            waker,
        })
    }

    pub fn ctl(&self, fd: RawFd, op: libc::c_int, events: Event, token: u64) -> Result<()> {
        if token == WAKER_TOKEN && fd != self.waker.as_raw_fd() {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        let mut event = libc::epoll_event {
            events: events.bits() as _,
            u64: token as u64,
        };
        if unsafe { libc::epoll_ctl(self.handle, op, fd, &mut event as _) } >= 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }

    fn wait(&self, events: &mut [EPollEvent], timeout_ms: i32) -> Result<i32> {
        let maxevents = if events.len() > i32::MAX as usize {
            i32::MAX
        } else {
            events.len() as i32
        };
        let status = unsafe {
            libc::epoll_pwait(
                self.handle,
                events.as_mut_ptr() as _,
                maxevents,
                timeout_ms,
                std::ptr::null() as _,
            )
        };
        if status >= 0 {
            // Filter out wake event
            for i in 0..status as usize {
                if events[i].token() == WAKER_TOKEN {
                    // drain eventfd
                    self.waker.read(100).expect("Unrecoverable error");
                    // remove by swapping out with the last one
                    events[i] = events[(status as usize) - 1];
                    return Ok(status - 1);
                }
            }
            Ok(status)
        } else {
            Err(Error::last_os_error())
        }
    }

    #[inline(always)]
    pub fn wake(&self) -> Result<()> {
        self.waker.write(1)
    }
}

impl Drop for EPoll {
    fn drop(&mut self) {
        if self.handle > -1 {
            super::ioutils::close_log_on_error(self.handle);
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EPollEvent(libc::epoll_event);

impl EPollEvent {
    pub fn token(&self) -> u64 {
        self.0.u64
    }
    pub fn events(&self) -> Event {
        Event::from_bits_truncate(self.0.events as _)
    }
}

impl std::fmt::Debug for EPollEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EPollEvent")
            .field("events", &self.events())
            .field("token", &self.token())
            .finish()
    }
}
