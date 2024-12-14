use num::ToPrimitive;
use std::i32;
use std::io::{Error, Result};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::IO::{
    GetQueuedCompletionStatusEx, PostQueuedCompletionStatus, OVERLAPPED_ENTRY,
};
use windows_sys::Win32::{
    Foundation::{BOOLEAN, HANDLE},
    System::IO::CreateIoCompletionPort,
};

use super::Event;

const WAKE_TOKEN: usize = 0usize;

#[derive(Debug)]
pub struct IoCompletionPort {
    handle: HANDLE,
    bound_events: Mutex<Vec<BoundEvent>>,
}
// FIXME: Drop

#[derive(Debug, Clone)]
struct BoundEvent {
    event: Event,
    overlapped: *const OVERLAPPED_ENTRY,
}

impl IoCompletionPort {
    pub fn new(max_number_of_threads: u32) -> Result<Self> {
        let bound_events: Mutex<Vec<BoundEvent>> = Mutex::new(Vec::default());
        let handle = {
            let handle = unsafe {
                CreateIoCompletionPort(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null_mut(),
                    0,
                    max_number_of_threads as _,
                )
            };
            if handle.is_null() {
                return Err(Error::last_os_error());
            }
            handle
        };
        Ok(Self {
            handle,
            bound_events,
        })
    }

    pub fn bind_event(&mut self, event: &Event) {
        let events = self.bound_events.lock();
        // events.
        todo!();
    }

    pub fn get_native_handle(&self) -> HANDLE {
        self.handle
    }

    pub fn wake(&self) -> Result<()> {
        if unsafe { PostQueuedCompletionStatus(self.handle, 0, WAKE_TOKEN, std::ptr::null_mut()) }
            != 0
        {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }

    pub fn wait(
        &self,
        timeout_ms: i32,
        events: &mut Vec<OVERLAPPED_ENTRY>,
    ) -> std::io::Result<i32> {
        let timeout_ms = if timeout_ms < 0 {
            0xFFFFFFFFu32
        } else {
            timeout_ms as u32
        };
        events.clear();
        let capacity = events.capacity().to_i32().unwrap_or(i32::MAX);
        let mut nentries: u32 = 0;
        if (unsafe {
            GetQueuedCompletionStatusEx(
                self.handle,
                events.as_mut_ptr(),
                capacity as _,
                &mut nentries as _,
                timeout_ms,
                0,
            )
        }) != 0
        {
            // This is safe because `GetQueuedCompletionStatusEx` ensures that `nentries` are assigned.
            unsafe { events.set_len(nentries as usize) };
            // Remove the wake tokens
            events.retain(|e| e.lpCompletionKey != WAKE_TOKEN);
            Ok(events.len() as _)
        } else {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            match err {
                windows_sys::Win32::Foundation::WAIT_TIMEOUT => Ok(0),
                _ => Err(Error::from_raw_os_error(err as _)),
            }
        }
    }
}
