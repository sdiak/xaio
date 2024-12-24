// use libc::{kevent, EV_ADD, EV_DELETE, EV_ERROR, EV_RECEIPT, EV_CLEAR, EVFILT_USER, NOTE_TRIGGER};
pub struct kevent {
    /// Identifier for this event (often a file descriptor)
    ident: libc::uintptr_t,
    filter: libc::c_short,
    flags: libc::c_ushort,
    fflags: libc::c_int,
    data: libc::intptr_t,
    udata: *mut libc::c_void,
}

pub const EV_ADD: u16 = 0x1;
pub const EV_DELETE: u16 = 0x2;
pub const EV_ERROR: u16 = 0x4000;
pub const EV_RECEIPT: u16 = 0x40;
pub const EV_CLEAR: u16 = 0x20;
pub const EVFILT_USER: i16 = -10;
pub const NOTE_TRIGGER: u32 = 0x01000000;

/*
EVFILT_USER:
 */

use std::{
    io::{Error, ErrorKind, Result},
    ptr::NonNull,
};

macro_rules! kevent_new {
    ($id: expr, $filter: expr, $flags: expr, $data: expr) => {
        kevent {
            ident: $id as _,
            filter: $filter as _,
            flags: $flags as _,
            udata: $data as _,
            ..unsafe { std::mem::zeroed() }
        }
    };
}
const WAKE_TOKEN: usize = usize::MAX;

unsafe extern "C" fn kevent(
    kq: libc::c_int,
    changelist: *const kevent,
    nchanges: libc::c_int,
    eventlist: *mut kevent,
    nevents: libc::c_int,
    timeout: *const libc::timespec,
) -> libc::c_int {
    -1
}
unsafe extern "C" fn kqueue() -> libc::c_int {
    -1
}

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "freebsd", target_os = "macos" ))] {
        pub struct KQueue(libc::c_int);
    } else {
        pub struct KQueue(libc::c_int, libc::c_int, libc::c_int);
    }
}

impl Drop for KQueue {
    fn drop(&mut self) {
        if self.0 >= 0 {
            super::ioutils::close_log_on_error(self.0);
        }
    }
}

impl std::os::fd::AsRawFd for KQueue {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0 as _
    }
}

impl KQueue {
    pub fn new() -> Result<Self> {
        let kq = unsafe { kqueue() };
        if kq >= 0 {
            cfg_if::cfg_if! {
                if #[cfg(any(target_os = "freebsd", target_os = "macos" ))] {
                    Self(kq).__init()
                } else {
                    if let Ok((r, w)) = super::ioutils::libc_pipe2(true, true) {
                        Self(kq, r, w).__init()
                    } else {
                        let err = Error::last_os_error();
                        super::ioutils::close_log_on_error(kq);
                        Err(err)
                    }
                }
            }
        } else {
            Err(Error::last_os_error())
        }
    }
    #[cfg(any(target_os = "freebsd", target_os = "macos"))]
    fn __evfilt_user(&self, fflags: i32) -> Result<()> {
        let mut ev: kevent = kevent_new!(0, EVFILT_USER, EV_ADD | EV_RECEIPT, WAKE_TOKEN);
        ev.fflags = fflags as _;
        let status = unsafe { kevent(self.0, &ev, 1, &mut ev, 1, std::ptr::null()) };
        if status >= 0 && (ev.flags & EV_ERROR) == 0 {
            Ok(())
        } else if status >= 0 {
            if ev.data != 0 {
                Err(Error::from_raw_os_error(ev.data as _))
            } else {
                Err(Error::from(ErrorKind::Other))
            }
        } else {
            Err(Error::last_os_error())
        }
    }
    fn __init(self) -> Result<Self> {
        // Setup waker
        cfg_if::cfg_if! {
            if #[cfg(any(target_os = "freebsd", target_os = "macos" ))] {
                self.__evfilt_user(0)?;
                Ok(self)
            } else {
                // TODO:
                Err(Error::from(ErrorKind::Unsupported))
                FIXME:
            }
        }
    }
    pub fn try_clone(&self) -> Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(any(target_os = "freebsd", target_os = "macos" ))] {
                Ok(Self(super::ioutils::dup(self.0)?))
            } else {
                let mut err: Option<Error> = None;
                let kq = super::ioutils::dup(self.0)?;
                if let Ok(r) = super::ioutils::dup(self.1) {
                    if let Ok(w) = super::ioutils::dup(self.2) {
                        return Ok(Self(kq, r, w))
                    }
                    err = Some(Error::last_os_error());
                    super::ioutils::close_log_on_error(r);
                }
                super::ioutils::close_log_on_error(kq);
                Err(err.unwrap_or(Error::last_os_error()))
            }
        }
    }
    pub fn wake(&self) -> Result<()> {
        cfg_if::cfg_if! {
            if #[cfg(any(target_os = "freebsd", target_os = "macos" ))] {
                self.__evfilt_user(NOTE_TRIGGER as _)?;
                Ok(self)
            } else {
                let buf = [0u8;1];
                super::ioutils::write_all(self.2, &buf, true)
            }
        }
    }

    pub fn kevent(
        &self,
        changes: &Vec<kevent>,
        events: &mut Vec<kevent>,
        timeout_ms: i32,
    ) -> Result<()> {
        if changes.len() > i32::MAX as _ || events.len() >= i32::MAX as _ {
            log::warn!(
                "KQueue::submit_and_wait(...), vectors lengthes can not be greater than {}",
                i32::MAX
            );
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        let mut timeout_buffer = libc::timespec {
            tv_sec: 0 as _,
            tv_nsec: 0 as _,
        };
        let timeout: *const libc::timespec = if timeout_ms < 0 {
            std::ptr::null()
        } else {
            timeout_buffer.tv_sec = (timeout_ms / 1_000) as _;
            timeout_buffer.tv_nsec = ((timeout_ms % 1_000) * 1_000_000) as _;
            &timeout_buffer as *const libc::timespec
        };
        // Make room for at least changes.len() event notifications
        if events.capacity() < changes.len()
            && events
                .try_reserve(changes.len() - events.capacity())
                .is_err()
        {
            return Err(Error::from(ErrorKind::OutOfMemory));
        }
        let n_events = unsafe {
            kevent(
                self.0,
                changes.as_ptr(),
                changes.len() as _,
                events.as_mut_ptr(),
                events.len() as _,
                timeout,
            )
        };
        if n_events >= 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
}
