use crate::utils::windows_close_handle_log_on_error;
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;
use windows_sys::Win32::Foundation::{HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{CreateEventA, SetEvent, WaitForSingleObject};

/// An event can be used as an event wait/notify mechanism by user-space applications, and by the kernel to notify user-space applications of events.
///
/// An event starts not-notified.
#[repr(C)]
#[derive(Debug)]
pub struct Event {
    handle: Arc<Inner>,
}
#[repr(C)]
#[derive(Debug, Clone)]
struct Inner {
    handle: HANDLE,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        windows_close_handle_log_on_error(self.handle);
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl Event {
    pub fn new() -> Result<Self> {
        let handle = unsafe { CreateEventA(std::ptr::null() as _, 0, 0, std::ptr::null() as _) };
        if handle.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(Self {
                handle: Arc::new(Inner { handle }),
            })
        }
    }
    /// Notify a waiter (multiple notification may be coalesced into one)
    pub fn notify(&self) -> Result<()> {
        if unsafe { SetEvent(self.get_native_handle()) } != 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }

    #[inline]
    pub unsafe fn get_native_handle(&self) -> HANDLE {
        (*self.handle).handle
    }

    /// Waits for the event to be notified or for `timeout_ms` milliseconds
    pub fn wait(&self, timeout_ms: i32) -> Result<()> {
        let timeout_ms = if timeout_ms < 0 {
            0xFFFFFFFFu32
        } else {
            timeout_ms as u32
        };
        match unsafe { WaitForSingleObject(self.get_native_handle(), timeout_ms) } {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_TIMEOUT => Err(Error::from(ErrorKind::TimedOut)),
            WAIT_ABANDONED => panic!(
                "Event handle ({:?}) is seen as a Mutex handle",
                *self.handle
            ),
            _ => Err(Error::last_os_error()),
        }
    }
}
