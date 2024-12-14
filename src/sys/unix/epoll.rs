use crate::selector::{Interest, RawSelectorHandle};
use crate::RawSocketFd;
use std::{
    io::{Error, ErrorKind, Result},
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
};

#[derive(Debug)]
pub struct EPoll {
    epfd: OwnedFd,
}

const _: () = assert!(
    std::mem::align_of::<crate::selector::Event>() >= std::mem::align_of::<libc::epoll_event>()
        && std::mem::size_of::<crate::selector::Event>()
            == std::mem::size_of::<libc::epoll_event>()
        && Interest::READABLE.bits() == libc::EPOLLIN as u32
        && Interest::WRITABLE.bits() == libc::EPOLLOUT as u32
        && Interest::PRIORITY.bits() == libc::EPOLLPRI as u32
        && Interest::ERROR.bits() == libc::EPOLLERR as u32
        && Interest::HANG_UP.bits() == libc::EPOLLHUP as u32
        && Interest::RDHANG_UP.bits() == libc::EPOLLRDHUP as u32
);
impl EPoll {
    pub fn new(close_on_exec: bool) -> Result<Self> {
        let epfd = unsafe {
            libc::epoll_create1(if close_on_exec {
                libc::EPOLL_CLOEXEC
            } else {
                0
            })
        };
        if epfd >= 0 {
            Ok(EPoll {
                epfd: unsafe { OwnedFd::from_raw_fd(epfd) },
            })
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn try_clone(&self) -> Result<Self> {
        let epfd = unsafe { libc::dup(self.epfd.as_raw_fd()) };
        if epfd >= 0 {
            Ok(EPoll {
                epfd: unsafe { OwnedFd::from_raw_fd(epfd) },
            })
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    fn epoll_ctl(&self, fd: RawFd, op: libc::c_int, events: u32, token: usize) -> Result<()> {
        let mut event = libc::epoll_event {
            events: events & !Interest::ONESHOT.bits(),
            u64: token as u64,
        };
        if unsafe { libc::epoll_ctl(self.epfd.as_raw_fd(), op, fd, &mut event as _) } >= 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
}

impl crate::selector::SelectorImpl for EPoll {
    fn register(
        &self,
        fd: RawSocketFd,
        token: usize,
        interests: crate::selector::Interest,
    ) -> Result<()> {
        self.epoll_ctl(
            fd.inner,
            libc::EPOLL_CTL_ADD,
            interests.bits() | libc::EPOLLET as u32 | libc::EPOLLEXCLUSIVE as u32,
            token,
        )
    }
    fn reregister(&self, fd: RawSocketFd, token: usize, interests: Interest) -> Result<()> {
        self.epoll_ctl(
            fd.inner,
            libc::EPOLL_CTL_MOD,
            interests.bits() | libc::EPOLLET as u32 | libc::EPOLLEXCLUSIVE as u32,
            token,
        )
    }
    fn unregister(&self, fd: RawSocketFd) -> Result<()> {
        self.epoll_ctl(fd.inner, libc::EPOLL_CTL_DEL, 0u32, 0usize)
    }
    fn select(&self, events: &mut Vec<crate::selector::Event>, timeout_ms: i32) -> Result<usize> {
        events.clear();
        if unsafe {
            libc::epoll_pwait(
                self.epfd.as_raw_fd(),
                events.as_mut_ptr() as _,
                events.capacity() as _,
                timeout_ms,
                std::ptr::null() as _,
            )
        } >= 0
        {
            Ok(0)
        } else {
            Err(Error::last_os_error())
        }
    }
    unsafe fn get_native_handle(&self) -> RawSelectorHandle {
        RawSelectorHandle::new(self.epfd.as_raw_fd())
    }
}
