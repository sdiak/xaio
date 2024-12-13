use crate::{
    libc_close_log_on_error, saturating_opt_duration_to_timespec,
    sys::{Event, EventCallBack},
    DriverConfig, DriverFlags, DriverHandle, DriverIFace, Request, RequestHandle,
};
use std::{
    io::{Error, ErrorKind, Result},
    time,
};

const BUFFER_SIZE: usize = 256usize;
const DRIVER_NAME: &str = "EPoll";
pub(crate) const WAKE_TOKEN: u64 = 0u64;

//https://doc.rust-lang.org/stable/core/mem/union.MaybeUninit.html#initializing-an-array-element-by-element

#[derive(Debug)]
pub struct DriverEPoll {
    epollfd: libc::c_int,
    waker: Event,
    config: DriverConfig,
    npending_events: usize,
}

impl DriverEPoll {
    pub(crate) fn new(config: &DriverConfig) -> Result<Self> {
        let waker = Event::new()?;
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.flags =
            config.flags & (DriverFlags::ATTACH_HANDLE | DriverFlags::CLOSE_ON_EXEC).bits();
        real_config.attach_handle = -1i32 as usize;
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        let epollfd: libc::c_int =
            if (real_config.flags & DriverFlags::ATTACH_HANDLE.bits()) != 0u32 {
                let epollfd = config.attach_handle as _; // TODO: dup() Reflect on the semantic of attach (epoll/kqueue/... => same events, iouring => shared kernel workers )
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
                let mut waker_reg = libc::epoll_event {
                    events: (libc::EPOLLIN | libc::EPOLLET) as _,
                    u64: WAKE_TOKEN,
                };
                let status = unsafe {
                    libc::epoll_ctl(
                        epollfd,
                        libc::EPOLL_CTL_ADD,
                        waker.get_native_handle(),
                        &mut waker_reg as _,
                    )
                };
                if status < 0 {
                    let err = Err(Error::last_os_error());
                    libc_close_log_on_error(epollfd);
                    return err;
                } else {
                    epollfd
                }
            };
        Ok(Self {
            epollfd,
            waker,
            config: real_config,
            npending_events: 0,
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
    fn submit(&mut self, _sub: std::pin::Pin<&mut Request>) -> Result<RequestHandle> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    fn cancel(&mut self, _handle: RequestHandle) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(
        &mut self,
        _ready_list: &mut crate::RequestList,
        timeout_ms: i32,
    ) -> std::io::Result<i32> {
        // if self.npending_events == 0 {
        //     timeout_ms = 0;
        // }
        let mut buffer: [libc::epoll_event; 64] =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        let n_events = unsafe {
            libc::epoll_pwait(
                self.epollfd,
                buffer.as_mut_ptr(),
                buffer.len() as _,
                timeout_ms,
                std::ptr::null(),
            )
        };
        if n_events < 0 {
            return Err(Error::last_os_error());
        }
        let mut n_user_events = 0;
        for ev in buffer {
            if ev.u64 != WAKE_TOKEN {
                n_user_events += 1;
            }
        }
        Ok(n_user_events)
    }
    #[inline]
    fn wake(&self) -> std::io::Result<()> {
        self.waker.notify()
    }
    #[inline]
    fn get_native_handle(&self) -> DriverHandle {
        self.epollfd
    }
}
