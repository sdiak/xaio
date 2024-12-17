pub enum EventCallBack {
    None,
    Rust(Box<dyn FnMut()>),
    C(extern "C" fn(*mut libc::c_void), *mut libc::c_void),
}

impl std::fmt::Debug for EventCallBack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventCallBack::None => f.write_str("EventCallBack::None"),
            Self::Rust(_) => f.write_str("EventCallBack::Rust(Box<dyn FnMut()>)"),
            Self::C(_, _) => f.write_str(
                "EventCallBack::Rust(extern \"C\" fn(*mut libc::c_void), *mut libc::c_void)",
            ),
        }
    }
}

#[cfg(target_family = "unix")]
mod unix;
use std::{fmt::Debug, usize};

#[cfg(target_family = "unix")]
pub use unix::*;

#[cfg(target_family = "windows")]
mod windows;
#[cfg(target_family = "windows")]
pub use windows::*;

pub mod poll;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(usize);
impl Debug for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_invalid() {
            f.write_str("ThreadId()")
        } else {
            f.write_fmt(format_args!(
                "ThreadId({:?})",
                self.0 as *const libc::c_void
            ))
        }
    }
}
impl ThreadId {
    #[inline(always)]
    pub fn current() -> Self {
        Self(__get_current_thread_id())
    }
    #[inline(always)]
    pub fn invalid() -> Self {
        Self(usize::MAX)
    }
    #[inline(always)]
    pub fn is_invalid(&self) -> bool {
        self.0 == usize::MAX
    }
    #[inline(always)]
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "linux", target_os = "freebsd")
    ))] {
        #[inline(always)]
        pub fn __get_current_thread_id() -> usize {
            use std::arch::asm;
            let mut id: usize;
            unsafe {
                asm!(
                    "mov {id}, fs:0",
                    id = lateout(reg) id,
                    options(nostack, pure, readonly),
                );
            }
            id as _

        }
    } else {
        #[inline(always)]
        fn __get_current_thread_id() -> usize {
            use std::cell::Cell;
            thread_local!(static ID: Cell<usize> = const { Cell::new(usize::MAX) });
            let mut id = ID.get();
            if id != usize::MAX {
                id
            } else {
                id = __next_thread_id();
                ID.set(id);
                id
            }
        }
        #[inline(never)]
        fn __next_thread_id() -> usize {
            static NEXT: std::sync::atomic::AtomicUsize  = std::sync::atomic::AtomicUsize::new(0usize);
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        }
    }
}
