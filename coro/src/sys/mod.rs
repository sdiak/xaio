use std::io::{Error, ErrorKind};
pub mod stack;
mod unix;

fn io_error_kind_to_errno_constant(err: ErrorKind) -> libc::c_int {
    // TODO:
    match err {
        ErrorKind::AlreadyExists => libc::EEXIST,
        _ => libc::EIO,
    }
}
#[cfg(target_family = "unix")]
pub fn io_error_to_errno_constant(err: &Error) -> libc::c_int {
    err.raw_os_error()
        .unwrap_or_else(|| io_error_kind_to_errno_constant(err.kind()))
}
#[cfg(not(target_family = "unix"))]
pub fn io_error_to_errno_constant(err: &Error) -> libc::c_int {
    err.raw_os_error()
        .unwrap_or_else(|| io_error_kind_to_errno_constant(err.kind()))
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(usize);
impl std::fmt::Debug for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_invalid() {
            f.write_str("ThreadId(-)")
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
