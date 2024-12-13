use crate::{
    saturating_opt_duration_to_ms, sys::Event, DriverConfig, DriverFlags, DriverHandle,
    DriverIFace, Request,
};
use std::{
    io::{Error, ErrorKind, Result},
    ptr::NonNull,
};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::IO::OVERLAPPED_ENTRY;
use windows_sys::Win32::System::IO::{
    CreateIoCompletionPort, GetQueuedCompletionStatusEx, PostQueuedCompletionStatus,
};

pub(crate) const WAKE_TOKEN: usize = 0usize;

const BUFFER_SIZE: usize = 256usize;

const DRIVER_NAME: &str = "IOCP";

pub struct DriverIOCP {
    iocp: HANDLE,
    waker: Event,
    config: DriverConfig,
    buffer: [OVERLAPPED_ENTRY; BUFFER_SIZE],
}
impl std::fmt::Debug for DriverIOCP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DriverIOCP(TODO:)")
    }
}
impl DriverIOCP {
    pub(crate) fn new(config: &DriverConfig) -> Result<Self> {
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.attach_handle = INVALID_HANDLE_VALUE as usize;
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        real_config.max_number_of_threads = num::clamp(config.max_number_of_fd_hint, 0, 65536);
        let waker = Event::new()?;

        let iocp = {
            let iocp = unsafe {
                CreateIoCompletionPort(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null_mut(),
                    0,
                    real_config.max_number_of_threads as _,
                )
            };
            if iocp.is_null() {
                return Err(Error::last_os_error());
            }
            iocp
        };
        Ok(Self {
            iocp,
            waker: waker,
            config: real_config,
            buffer: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
        })
    }

    fn wake(&self) -> Result<()> {
        if unsafe { PostQueuedCompletionStatus(self.iocp, 0, WAKE_TOKEN, std::ptr::null_mut()) }
            != 0
        {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
    fn process_events(&mut self, nevents: usize) -> i32 {
        let mut nuser_events = 0i32;
        for i in 0usize..nevents {
            let req = self.buffer[i].lpOverlapped as *mut Request;
            if !req.is_null() {
                // if (*req).owner
                todo!();
            }
        }
        nuser_events
    }
}

impl Drop for DriverIOCP {
    fn drop(&mut self) {
        if self.iocp != INVALID_HANDLE_VALUE {
            if unsafe { CloseHandle(self.iocp) } == 0 {
                log::warn!(
                    "DriverIOCP::drop(): failed closing the Iocp handle {:?}: {:?}",
                    self.iocp,
                    std::io::Error::last_os_error()
                );
            }
            self.iocp = INVALID_HANDLE_VALUE as _;
        }
    }
}

impl DriverIFace for DriverIOCP {
    fn config(&self) -> &DriverConfig {
        &self.config
    }
    #[inline]
    fn name(&self) -> &'static str {
        DRIVER_NAME
    }
    unsafe fn submit(&mut self, _req: NonNull<Request>) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    unsafe fn cancel(&mut self, _req: NonNull<Request>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(
        &mut self,
        _ready_list: &mut crate::RequestList,
        timeout_ms: i32,
    ) -> std::io::Result<i32> {
        let timeout_ms = if timeout_ms < 0 {
            0xFFFFFFFFu32
        } else {
            timeout_ms as u32
        };
        let mut nentries: u32 = 0;
        if (unsafe {
            GetQueuedCompletionStatusEx(
                self.iocp,
                self.buffer.as_mut_ptr(),
                BUFFER_SIZE as _,
                &mut nentries as _,
                timeout_ms,
                0,
            )
        }) != 0
        {
            Ok(self.process_events(nentries as _))
        } else {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            match err {
                windows_sys::Win32::Foundation::WAIT_TIMEOUT => Ok(0),
                _ => Err(Error::from_raw_os_error(err as _)),
            }
        }
    }
    #[inline]
    fn wake(&self) -> Result<()> {
        self.waker.notify()
    }
    #[inline]
    unsafe fn get_native_handle(&self) -> DriverHandle {
        self.iocp
    }
}
