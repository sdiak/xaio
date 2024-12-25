use libc::{kevent, EVFILT_READ, EV_ADD, EV_CLEAR, EV_DELETE, EV_ERROR, EV_RECEIPT, NOTE_TRIGGER};

#[cfg(has_evfilt_user)]
use libc::EVFILT_USER;

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

cfg_if::cfg_if! {
    if #[cfg(has_evfilt_user)] {
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
                if #[cfg(has_evfilt_user)] {
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
    #[cfg(has_evfilt_user)]
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
                Ok(())
            }
        } else {
            Err(Error::last_os_error())
        }
    }
    fn __init(self) -> Result<Self> {
        // Setup waker
        cfg_if::cfg_if! {
            if #[cfg(has_evfilt_user)] {
                self.__evfilt_user(0)?;
                Ok(self)
            } else {
                let mut ev = kevent_new!(self.1, EVFILT_READ, EV_ADD | EV_RECEIPT | EV_CLEAR, WAKE_TOKEN);
                self.__raw_kevent(&ev, 1, &mut ev, 1, -1)?;

                let status = unsafe { kevent(self.0, &ev, 1, &mut ev, 1, std::ptr::null()) };
                if status >= 0 && (ev.flags & EV_ERROR) == 0 {
                    Ok(self)
                } else if status >= 0 {
                    if ev.data != 0 {
                        Err(Error::from_raw_os_error(ev.data as _))
                    } else {
                        Ok(self)
                    }
                } else {
                    Err(Error::last_os_error())
                }
            }
        }
    }
    pub fn try_clone(&self) -> Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(has_evfilt_user)] {
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
            if #[cfg(has_evfilt_user)] {
                self.__evfilt_user(NOTE_TRIGGER as _)?;
                Ok(self)
            } else {
                let buf = [0u8;1];
                super::ioutils::write_all(self.2, &buf, true)
            }
        }
    }

    fn __raw_kevent(
        &self,
        changelist: *const kevent,
        nchanges: usize,
        eventlist: *mut kevent,
        nevents: usize,
        timeout_ms: i32,
    ) -> Result<i32> {
        if nchanges > i32::MAX as _ || nevents >= i32::MAX as _ {
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
        let n_events = unsafe {
            kevent(
                self.0,
                changelist,
                nchanges as _,
                eventlist,
                nevents as _,
                timeout,
            )
        };
        if n_events >= 0 {
            Ok(n_events)
        } else {
            let err_code = super::last_os_error();
            if err_code == libc::EINTR {
                // https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2#end
                // When  kevent()  call  fails  with  EINTR	 error,	 all  changes  in  the
                // changelist have been applied.
                Ok(0)
            } else {
                Err(Error::from_raw_os_error(err_code))
            }
        }
    }
    pub fn kevent(
        &self,
        changes: &Vec<kevent>,
        events: &mut Vec<kevent>,
        timeout_ms: i32,
    ) -> Result<()> {
        // Make room for at least changes.len() event notifications
        if events.try_reserve(changes.len()).is_err() {
            return Err(Error::from(ErrorKind::OutOfMemory));
        }
        let initial_events_len = events.len();
        self.__raw_kevent(
            changes.as_ptr(),
            changes.len(),
            unsafe { events.as_mut_ptr().offset(initial_events_len as isize) },
            events.capacity(),
            timeout_ms,
        )
        .map(|nevents| unsafe { events.set_len(initial_events_len + nevents as usize) })
    }
}
