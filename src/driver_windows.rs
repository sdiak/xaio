use crate::{saturating_opt_duration_to_ms, DriverConfig, DriverFlags, DriverIFace, Request};
use std::io::{Error, ErrorKind, Result};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::IO::OVERLAPPED_ENTRY;
use windows_sys::Win32::System::IO::{
    CreateIoCompletionPort, GetQueuedCompletionStatusEx, PostQueuedCompletionStatus,
};

pub(crate) const WAKE_TOKEN: usize = 0usize;

const BUFFER_SIZE: usize = 256usize;

const DRIVER_NAME: &'static str = "DriverWindows";

pub struct DriverWindows {
    iocp: HANDLE,
    config: DriverConfig,
    buffer: [OVERLAPPED_ENTRY; BUFFER_SIZE],
}
impl DriverWindows {
    fn new(config: &DriverConfig) -> Result<Self> {
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.flags = config.flags & (DriverFlags::ATTACH_HANDLE).bits();
        real_config.attach_handle = INVALID_HANDLE_VALUE as usize;
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        real_config.max_number_of_threads = num::clamp(config.max_number_of_fd_hint, 0, 65536);
        let iocp = if (real_config.flags & DriverFlags::ATTACH_HANDLE.bits()) != 0u32 {
            // TODO: how to get the max_number_of_threads ?
            let iocp: HANDLE = config.attach_handle as _;
            if iocp == INVALID_HANDLE_VALUE {
                return Err(Error::from(ErrorKind::InvalidInput));
            }
            real_config.flags = config.flags;
            real_config.attach_handle = config.attach_handle;
            iocp
        } else {
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

impl Drop for DriverWindows {
    fn drop(&mut self) {
        if self.iocp != INVALID_HANDLE_VALUE {
            if unsafe { CloseHandle(self.iocp) } == 0 {
                log::warn!(
                    "DriverWindows::drop(): failed closing the Iocp handle {:?}: {:?}",
                    self.iocp,
                    std::io::Error::last_os_error()
                );
            }
            self.iocp = INVALID_HANDLE_VALUE as _;
        }
    }
}

impl DriverIFace for DriverWindows {
    fn config(&self) -> &DriverConfig {
        &self.config
    }
    fn name(&self) -> &'static str {
        DRIVER_NAME
    }
    fn submit(&mut self, _sub: std::pin::Pin<&mut Request>) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    fn cancel(&mut self, _sub: std::pin::Pin<&Request>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(
        &mut self,
        timeout: Option<std::time::Duration>,
        _ready_list: &mut crate::RequestList,
    ) -> std::io::Result<i32> {
        let timeout_ms = saturating_opt_duration_to_ms(timeout);
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
}
