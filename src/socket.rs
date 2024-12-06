
#[cfg(not(target_os = "windows"))]
use std::os::fd::RawFd as Inner;
#[cfg(target_os = "windows")]
use std::os::windows::raw::SOCKET as Inner;

#[derive(Copy, PartialEq, Eq, Clone, PartialOrd, Ord, Hash)]
pub struct RawSocketFd {
    pub(crate) inner: Inner
}

impl RawSocketFd {
    pub fn new(fd: Inner) -> Self {
        Self { inner: fd }
    }
    pub fn invalid() -> Self {
        if cfg!(windows) {
            Self { inner: winapi::um::winsock2::INVALID_SOCKET as Inner }
        } else {
            Self { inner: -1i32 as Inner }
        }
    }
    pub fn is_valid(&self) -> bool {
        if cfg!(windows) {
            self.inner != (winapi::um::winsock2::INVALID_SOCKET as Inner)
        } else {
            self.inner >= (0 as Inner)
        }
    }
}
