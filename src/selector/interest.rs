use bitflags::bitflags;
use libc;
use std::num::NonZeroU32;
use std::{fmt, ops};

#[cfg(target_os = "linux")]
const READABLE: u32 = libc::EPOLLIN as u32;
#[cfg(not(target_os = "linux"))]
const READABLE: u32 = 0x001u32;

#[cfg(target_os = "linux")]
const PRIORITY: u32 = libc::EPOLLPRI as u32;
#[cfg(not(target_os = "linux"))]
const PRIORITY: u32 = 0x002u32;

#[cfg(target_os = "linux")]
const WRITABLE: u32 = libc::EPOLLOUT as u32;
#[cfg(not(target_os = "linux"))]
const WRITABLE: u32 = 0x004u32;

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Interest: u32 {
        /// Interest interests.
        const READABLE = READABLE;
        /// Writable interests.
        const WRITABLE = WRITABLE;
        /// Priority interests.
        const PRIORITY = PRIORITY;
    }
}
