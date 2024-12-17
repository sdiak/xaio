use super::{EPoll, Event, EventBuffer, URing};
use crate::capi::xconfig_s;
use crate::selector::Interest;
use crate::selector::SelectorImpl;
use num;
use std::fmt::Debug;
use std::io::Result;

const WAKER_TOKEN: usize = 0;

#[derive(Debug)]
pub struct Driver {
    ring: URing,
    epoll: EPoll,                   // -1 when polling using ring
    waker: Event,                   // TODO: register
    waker_buffer: Box<EventBuffer>, // TODO: pipe is only 1 byte
    config: crate::capi::xconfig_s,
}

impl Driver {
    pub fn new(config_hints: &crate::capi::xconfig_s) -> Result<Self> {
        let waker_buffer: Box<EventBuffer> = crate::catch_enomem(|| Box::new(0 as _))?;
        let mut config = *config_hints;
        config.submission_queue_depth = num::clamp(config.submission_queue_depth, 16, 4096);
        if config.completion_queue_depth < config.submission_queue_depth * 2 {
            config.completion_queue_depth = config.submission_queue_depth * 2;
        } else {
            config.completion_queue_depth = num::clamp(config.completion_queue_depth, 16, 4096);
        }
        config.flags = config.flags
            & (crate::capi::XCONFIG_FLAG_FAST_POLL
                | crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC
                | crate::capi::XCONFIG_FLAG_ATTACH_SINGLE_ISSUER
                | crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC);
        let waker = Event::new()?;
        let mut ring = URing::invalid();
        let probe = &*super::PROBE;
        let mut epoll = EPoll::invalid();
        if probe.is_supported() {
            ring = URing::new(&mut config, probe)?;
            if (config.flags & crate::capi::XCONFIG_FLAG_FAST_POLL) != 0 {
                epoll = EPoll::new((config.flags & crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC) != 0)?;
            }
        } else {
            epoll = EPoll::new((config.flags & crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC) != 0)?;
        }
        (Self {
            ring,
            epoll,
            waker,
            waker_buffer,
            config,
        })
        .register_waker()
    }
    pub fn default() -> Result<Self> {
        Self::new(&xconfig_s::default())
    }

    // pub fn wake(&)
    fn register_waker(mut self) -> Result<Self> {
        if self.ring.is_valid() {
            self.ring.add_sqe(|mut sqe| {
                sqe.prep_read(
                    unsafe { self.waker.get_native_handle() },
                    self.waker_buffer.as_mut() as *mut EventBuffer as _,
                    std::mem::size_of::<EventBuffer>() as _,
                    0,
                    WAKER_TOKEN,
                );
                Ok(())
            })?;
        } else {
            self.epoll.register(
                crate::RawSocketFd::new(unsafe { self.waker.get_native_handle() }),
                WAKER_TOKEN,
                Interest::READABLE,
            )?;
        }
        Ok(self)
    }
}
