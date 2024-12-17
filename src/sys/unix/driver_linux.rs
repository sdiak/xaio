use super::{EPoll, Event, EventBuffer, URing};
use crate::capi::xconfig_s;
use crate::selector::Interest;
use crate::selector::SelectorImpl;
use num;
use std::fmt::Debug;
use std::io::Result;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

const WAKER_TOKEN: usize = 0;

#[derive(Debug)]
pub struct Driver {
    inner: Box<Inner>,
}

#[derive(Debug)]
struct Inner {
    ring: URing,
    epoll: EPoll,                   // -1 when polling using ring
    waker: Event,                   // TODO: register
    waker_buffer: Box<EventBuffer>, // TODO: pipe is only 1 byte
    config: crate::capi::xconfig_s,
    need_init: AtomicBool,
    _pin: std::marker::PhantomPinned,
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
        Ok(Self {
            inner: Box::new(Inner {
                ring,
                epoll,
                waker,
                waker_buffer,
                config,
                need_init: AtomicBool::new(true),
                _pin: std::marker::PhantomPinned {},
            }),
        })
    }
    pub fn default() -> Result<Self> {
        Self::new(&xconfig_s::default())
    }

    #[inline(always)]
    pub fn init(&self) -> Result<()> {
        // FIXME: remove pub
        if self.inner.need_init.load(Ordering::Relaxed) {
            self.init_slow_path()
        } else {
            Ok(())
        }
    }
    #[inline(never)]
    fn init_slow_path(&self) -> Result<()> {
        let thiz = self.inner.as_ref();
        let fd = unsafe { thiz.waker.get_native_handle() };
        if thiz.ring.is_valid() {
            let buffer = thiz.waker_buffer.as_ref() as *const EventBuffer as usize;
            // let mut buffer = ManuallyDrop::new(Box::new(0u64)); buffer.as_mut() as *mut EventBuffer as usize
            thiz.ring.submit_batch(|mut sqes| {
                sqes.next()?.prep_read(
                    fd,
                    buffer as _,
                    std::mem::size_of::<EventBuffer>() as _,
                    0,
                    WAKER_TOKEN,
                );
                Ok(sqes)
            })?;
            println!("ICI\n");
        } else {
            thiz.epoll
                .register(crate::RawSocketFd::new(fd), WAKER_TOKEN, Interest::READABLE)?;
        }
        thiz.need_init.store(false, Ordering::Relaxed);
        println!("LA\n");
        Ok(())
    }

    pub fn wake(&self) -> Result<()> {
        self.inner.waker.notify()
    }
}
