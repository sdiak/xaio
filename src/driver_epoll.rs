use crate::{saturating_opt_duration_to_timespec, DriverConfig, DriverFlags, DriverIFace, Request};
use std::io::{Error, ErrorKind, Result};

const BUFFER_SIZE: usize = 256usize;

#[derive(Debug)]
pub struct DriverEPoll {
    epollfd: libc::c_int,
    config: DriverConfig,
    buffer: [libc::epoll_event; BUFFER_SIZE],
}

impl DriverEPoll {
    fn new(config: &DriverConfig) -> Result<Self> {
        let mut epollfd: libc::c_int = -1 as _;
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.flags =
            config.flags & (DriverFlags::ATTACH_HANDLE | DriverFlags::CLOSE_ON_EXEC).bits();
        real_config.attach_handle = -1i32 as usize;
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        if (real_config.flags & DriverFlags::ATTACH_HANDLE.bits()) != 0u32 {
            epollfd = config.attach_handle as _;
            if epollfd <= 0 {
                return Err(Error::from(ErrorKind::InvalidInput));
            }
        } else {
            epollfd = unsafe {
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
        }
        Ok(Self {
            epollfd: epollfd,
            config: real_config,
            buffer: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
        })
    }
    fn process_events(&mut self, nevents: usize) -> i32 {
        let mut nuser_events = 0i32;
        for i in 0usize..nevents {
            todo!();
        }
        nuser_events
    }
}

impl Drop for DriverEPoll {
    fn drop(&mut self) {
        if self.epollfd >= 0 {
            if unsafe { libc::close(self.epollfd) } < 0 {
                log::warn!(
                    "xepoll_close: failed closing the epoll file descriptor {}: {:?}",
                    self.epollfd,
                    std::io::Error::last_os_error()
                );
            }
            self.epollfd = -1 as _;
        }
    }
}

impl DriverIFace for DriverEPoll {
    fn config(&self) -> &DriverConfig {
        &self.config
    }
    fn name(&self) -> &'static str {
        "DriverEPoll"
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
}
