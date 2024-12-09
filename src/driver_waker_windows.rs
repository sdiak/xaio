use std::io::{Error, Result};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::IO::PostQueuedCompletionStatus;
pub(crate) struct DriverWaker {
    handle: HANDLE,
}
impl DriverWaker {
    pub(crate) fn new(driver_handle: HANDLE) -> Self {
        Self {
            handle: driver_handle,
        }
    }
    pub(crate) fn wake(&self) -> Result<()> {
        if unsafe {
            PostQueuedCompletionStatus(
                self.handle,
                0,
                super::driver_windows::WAKE_TOKEN,
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
