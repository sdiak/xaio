use crate::{
    driver_waker::DriverWaker, libc_close_log_on_error, saturating_opt_duration_to_timespec,
    DriverConfig, DriverFlags, DriverHandle, DriverIFace, Request,
};
use std::io::{Error, ErrorKind, Result};

const BUFFER_SIZE: usize = 256usize;
const DRIVER_NAME: &str = "EPoll";

#[derive(Debug)]
pub struct DriverEPoll {
    epollfd: libc::c_int,
    waker: DriverWaker,
    config: DriverConfig,
    buffer: [libc::epoll_event; BUFFER_SIZE],
}

impl DriverEPoll {
    pub(crate) fn new(config: &DriverConfig) -> Result<Self> {
        let waker = DriverWaker::new()?;
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.flags =
            config.flags & (DriverFlags::ATTACH_HANDLE | DriverFlags::CLOSE_ON_EXEC).bits();
        real_config.attach_handle = -1i32 as usize;
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        let epollfd: libc::c_int =
            if (real_config.flags & DriverFlags::ATTACH_HANDLE.bits()) != 0u32 {
                let epollfd = config.attach_handle as _;
                if epollfd <= 0 {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
                epollfd
            } else {
                let epollfd = unsafe {
                    libc::epoll_create1(
                        if (real_config.flags & DriverFlags::CLOSE_ON_EXEC.bits()) != 0 {
                            libc::EPOLL_CLOEXEC
                        } else {
                            0
                        },
                    )
                };
                if epollfd < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                epollfd
            };
        Ok(Self {
            epollfd,
            waker,
            config: real_config,
            buffer: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
        })
    }
    fn process_events(&mut self, nevents: usize) -> i32 {
        let mut nuser_events = 0i32;
        for i in 0usize..nevents {
            nuser_events += i as i32; // FIXME:
            todo!();
        }
        nuser_events
    }
}

impl Drop for DriverEPoll {
    fn drop(&mut self) {
        libc_close_log_on_error(self.epollfd);
    }
}

impl DriverIFace for DriverEPoll {
    fn config(&self) -> &DriverConfig {
        &self.config
    }
    #[inline]
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
        let mut ts_mem = libc::timespec {
            tv_nsec: 0 as _,
            tv_sec: 0 as _,
        };
        let n_events = unsafe {
            libc::epoll_pwait2(
                self.epollfd,
                self.buffer.as_mut_ptr(),
                BUFFER_SIZE as libc::c_int,
                saturating_opt_duration_to_timespec(timeout, &mut ts_mem), // SAFETY: ts_mem is live for the whole function
                std::ptr::null(),
            )
        };
        if n_events < 0 {
            return Err(Error::last_os_error());
        }
        Ok(self.process_events(n_events as usize))
    }
    #[inline]
    fn wake(&self) -> std::io::Result<()> {
        self.waker.wake()
    }
    #[inline]
    fn get_native_handle(&self) -> DriverHandle {
        self.epollfd
    }
}
