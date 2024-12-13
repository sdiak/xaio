use std::time::Duration;

mod poll;
pub use poll::*;
mod event;
pub use event::*;
mod interest;
use crate::RawSocketFd;
pub use interest::*;

#[cfg(not(target_family = "windows"))]
const AN_INVALID_SELECTOR_HANDLE: libc::c_int = -1 as _;
#[cfg(target_family = "windows")]
const AN_INVALID_SELECTOR_HANDLE: windows_sys::Win32::Foundation::HANDLE =
    windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

#[derive(Debug, Clone, Copy)]
pub struct RawSelectorHandle {
    #[cfg(not(target_family = "windows"))]
    raw: libc::c_int,
    #[cfg(target_family = "windows")]
    raw: windows_sys::Win32::Foundation::HANDLE,
}

impl RawSelectorHandle {
    #[cfg(not(target_family = "windows"))]
    pub fn new(raw: libc::c_int) -> Self {
        Self {
            raw: if raw < 0 { -1 } else { raw },
        }
    }
    #[cfg(target_family = "windows")]
    pub fn new(raw: windows_sys::Win32::Foundation::HANDLE) -> Self {
        Self { raw }
    }

    pub fn new_invalid() -> Self {
        Self {
            raw: AN_INVALID_SELECTOR_HANDLE,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.raw != AN_INVALID_SELECTOR_HANDLE
    }

    #[cfg(not(target_family = "windows"))]
    pub unsafe fn raw(&self) -> libc::c_int {
        self.raw
    }
    #[cfg(target_family = "windows")]
    pub unsafe fn raw(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.raw
    }
}

pub trait SelectorImpl {
    fn register(&self, fd: RawSocketFd, token: usize, interests: Interest) -> std::io::Result<()>;
    fn reregister(&self, fd: RawSocketFd, token: usize, interests: Interest)
        -> std::io::Result<()>;
    fn unregister(&self, fd: RawSocketFd) -> std::io::Result<()>;
    fn select(&self, events: &mut Vec<Event>, timeout_ms: i32) -> std::io::Result<usize>;
    unsafe fn get_native_handle(&self) -> RawSelectorHandle;
}

pub struct Selector {}
