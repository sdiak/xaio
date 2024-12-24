use super::RawFd;
use super::RawSd;
use crate::PollFlag;
cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        use libc::epoll_event as sys_event;
    } else if #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "dragonfly", target_os = "openbsd", target_os = "netbsd"))] {
            use libc::{EV_ADD, EV_DELETE, EV_ERROR, kevent as kevent};
    } else {
        #[repr(C, packed)]
        struct sys_event {
            events: u16,
            
            u64: u64,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SelectorEvent(sys_event);

impl SelectorEvent {
    pub fn token(&self) -> usize {
        self.0.u64 as _
    }
    pub fn flags(&self) -> PollFlag {
        PollFlag::from_bits_truncate(self.0.events as _)
    }
}

pub struct ChangeEvent {
    /// Identifier for this event (often a file descriptor)
    ident: libc::uintptr_t,
    filter: libc::c_short,
    flags: libc::c_ushort,
    fflags: libc::c_int,
    data: libc::intptr_t,
    udata: *mut libc::c_void,
};
impl ChangeEvent {
    const PUT: u32 = 1u32 << 30;
    const DEL: u32 = 1u32 << 31;

    fn put(fd: RawSd, token: usize, interests: PollFlag) -> ChangeEvent {
        let interests = interests & PollFlag::INTEREST_MASK;
        ChangeEvent(sys_event {
            u64: token as u64,
            events: ChangeEvent::PUT | interests.bits() as u32,
        })
    }
}

pub trait SelectorIFace {
    fn submit_and_wait(
        &self,
        changes: &Vec<SelectorEvent>,
        events: &mut Vec<SelectorEvent>,
        timeout_ms: i32,
    ) -> std::io::Result<()>;
    fn register(&self, fd: RawSd, token: usize, interests: PollFlag) -> std::io::Result<()>;
    fn reregister(&self, fd: RawSd, token: usize, interests: PollFlag) -> std::io::Result<()>;
    fn unregister(&self, fd: RawSd) -> std::io::Result<()>;
    fn select(&self, events: &mut Vec<SelectorEvent>, timeout_ms: i32) -> std::io::Result<()>;
    fn wake(&self);
    unsafe fn get_native_fd(&self) -> Option<RawFd> {
        None
    }
}
