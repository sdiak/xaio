use std::io::{Error, ErrorKind, Result};
use windows_sys::Win32::Foundation::{
    HANDLE, INVALID_HANDLE_VALUE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::{CreateEventA, SetEvent, WaitForSingleObject};
use windows_sys::Win32::System::IO::{CreateIoCompletionPort, PostQueuedCompletionStatus};
pub(crate) struct DriverWaker {
    handle: HANDLE,
}
/*
https://stackoverflow.com/a/78909504


Associate the Event object with IOCP, and get I/O Completion when it gets signalled, using [NtAssociateWaitCompletionPacket](https://learn.microsoft.com/en-us/windows/win32/devnotes/ntassociatewaitcompletionpacket). It's supported on Windows 8 and later.

See example here: https://github.com/tringi/win32-iocp-events

 */
impl DriverWaker {
    pub(crate) fn new() -> Result<Self> {
        let handle = unsafe { CreateEventA(std::ptr::null() as _, 0, 0, std::ptr::null() as _) };
        if handle.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(Self { handle: handle })
        }
    }
    pub(crate) fn wake(&self) -> Result<()> {
        if unsafe { SetEvent(self.handle) } != 0 {
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
    pub(crate) fn wait(&self, timeout_ms: i32) -> bool {
        let timeout_ms = if timeout_ms < 0 {
            0xFFFFFFFFu32
        } else {
            timeout_ms as u32
        };
        match unsafe { WaitForSingleObject(self.handle, timeout_ms) } {
            WAIT_OBJECT_0 => true,
            WAIT_TIMEOUT => false,
            WAIT_ABANDONED => panic!("Event handle is seen as a Mutex handle"),
            _ => {
                log::warn!(
                    "Unexepected error in `DriverWaker::wait(&self, timeout_ms={timeout_ms}): {}",
                    Error::last_os_error()
                );
                false
            }
        }
    }
}
