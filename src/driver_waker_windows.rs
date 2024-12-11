use std::io::{Error, Result};
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::IO::PostQueuedCompletionStatus;
pub(crate) struct DriverWaker {
    handle: HANDLE,
}
impl DriverWaker {
    pub(crate) fn new() -> Result<Self> {
        let handle =
            unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 0) };
        if handle.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(Self { handle: handle })
        }
    }
    pub(crate) fn wake(&self) -> Result<()> {
        if unsafe {
            PostQueuedCompletionStatus(
                self.handle,
                0,
                super::driver_iocp_windows::WAKE_TOKEN,
                std::ptr::null_mut(),
            )
        } != 0
        {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
    #[inline]
    pub(crate) fn read_end(&self) -> HANDLE {
        self.handle
    }
    #[inline]
    pub(crate) fn drain(&self) {}
}
