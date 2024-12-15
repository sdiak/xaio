use crate::{
    request,
    selector::{Interest, SelectorImpl},
    sys::{EPoll, Event},
    DriverConfig, DriverFlags, DriverHandle, DriverIFace, RawSocketFd, Request,
};
use std::{
    io::{Error, ErrorKind, Result},
    ptr::NonNull,
};

const BUFFER_SIZE: usize = 256usize;
const DRIVER_NAME: &str = "EPoll";
pub(crate) const WAKE_TOKEN: usize = 0usize;

//https://doc.rust-lang.org/stable/core/mem/union.MaybeUninit.html#initializing-an-array-element-by-element

#[derive(Debug)]
pub struct DriverEPoll {
    epoll: EPoll,
    waker: Event,
    config: DriverConfig,
    npending_events: usize,
    buffer: Vec<crate::selector::Event>,
}

impl DriverEPoll {
    pub(crate) fn new(config: &DriverConfig) -> Result<Self> {
        let waker = Event::new()?;
        let buffer: Vec<crate::selector::Event> =
            match std::panic::catch_unwind(|| Vec::with_capacity(BUFFER_SIZE)) {
                Ok(vec) => Ok(vec),
                Err(_) => Err(Error::from(ErrorKind::OutOfMemory)),
            }?;
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.flags =
            config.flags & (DriverFlags::ATTACH_HANDLE | DriverFlags::CLOSE_ON_EXEC).bits();
        real_config.attach_handle = -1i32 as usize;
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        // TODO: dup() on ATTACH_HANDLE
        // Before: Reflect on the semantic of attach (epoll/kqueue/... => same events, iouring => shared kernel workers )
        let epoll = EPoll::new(0u32 != (real_config.flags & DriverFlags::CLOSE_ON_EXEC.bits()))?;

        epoll.register(
            RawSocketFd::new(unsafe { waker.get_native_handle() }),
            WAKE_TOKEN,
            Interest::READABLE,
        )?;
        Ok(Self {
            epoll,
            waker,
            config: real_config,
            npending_events: 0,
            buffer,
        })
    }
    fn process_events(&mut self, nevents: usize) -> i32 {
        let mut nuser_events = 0i32;
        for _ in 0usize..nevents {
            nuser_events += 1 as i32; // FIXME:
            todo!();
        }
        nuser_events
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
    unsafe fn submit(&mut self, mut req: NonNull<Request>) -> Result<()> {
        let req = unsafe { req.as_mut() };
        match req.opcode_raw() {
            request::OP_NOOP => Err(Error::from(ErrorKind::Unsupported)),
            _ => Err(Error::from(ErrorKind::Unsupported)),
        }
    }
    unsafe fn cancel(&mut self, _req: NonNull<Request>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(&mut self, _ready_list: &mut crate::ReadyList, timeout_ms: i32) -> std::io::Result<()> {
        let mut n_user_events = 0i32;
        self.epoll.select(&mut self.buffer, timeout_ms)?;
        // for ev in self.buffer.iter() {
        //     if ev.token as usize != WAKE_TOKEN {
        //         n_user_events += 1;
        //     }
        // }
        Ok(())
    }
    #[inline]
    fn wake(&self) -> std::io::Result<()> {
        self.waker.notify()
    }
    #[inline]
    unsafe fn get_native_handle(&self) -> DriverHandle {
        self.epoll.get_native_handle().raw()
    }
}
