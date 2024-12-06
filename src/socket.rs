
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

    #[cfg(not(target_os = "windows"))]
    pub fn invalid() -> Self {
        Self { inner: -1i32 as Inner }
    }
    #[cfg(target_os = "windows")]
    pub fn invalid() -> Self {
        Self { inner: winapi::um::winsock2::INVALID_SOCKET as Inner }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_valid(&self) -> bool {
        self.inner >= (0 as Inner)
    }
    #[cfg(target_os = "windows")]
    pub fn is_valid(&self) -> bool {
        self.inner != (winapi::um::winsock2::INVALID_SOCKET as Inner)
    }
}
