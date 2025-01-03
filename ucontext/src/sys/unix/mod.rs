use std::ptr::NonNull;

#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
pub mod asm;

cfg_if::cfg_if! {
    if #[cfg(not(any(
        target_os = "openbsd",
        target_os = "macos",
        target_os = "ios",
        target_os = "android",
        target_os = "illumos",
        target_os = "solaris"
    )))] {
        const MMAP_FLAGS: libc::c_int = libc::MAP_PRIVATE | libc::MAP_ANON;
    } else {
        const MMAP_FLAGS: libc::c_int = libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_STACK;
    }
}
const MMAP_PROT: libc::c_int = libc::PROT_READ | libc::PROT_WRITE;

pub(crate) fn stack_alloc(total_size: usize, guard_size: usize) -> Option<NonNull<u8>> {
    let base = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            total_size,
            MMAP_PROT,
            MMAP_FLAGS,
            -1,
            0,
        )
    };

    if base == libc::MAP_FAILED {
        None
    } else {
        let mut base = base as *mut u8;
        let mut guard = base;
        let top = base;
        let top = unsafe { top.offset(total_size as isize) };
        if crate::sys::stack_growth_downward() {
            base = unsafe { base.offset(guard_size as isize) };
        } else {
            guard = unsafe { guard.offset((total_size - guard_size) as isize) }
        }
        println!(
            "stack_alloc() guarq:{:?}, bottom:{:?}, bottom-guard:{:?}, top:{:?}, top-bottom:{:?}",
            guard,
            base,
            (base as isize) - (guard as isize),
            top,
            (top as isize) - (base as isize),
        );
        assert!(unsafe { libc::mprotect(guard as _, guard_size, libc::PROT_NONE) } >= 0);
        Some(unsafe { NonNull::new_unchecked(base) })
    }
}

pub(crate) fn stack_dealloc(total_size: usize, guard_size: usize, base: NonNull<u8>) {
    let mut base = base.as_ptr();
    if crate::sys::stack_growth_downward() {
        base = unsafe { base.offset(-(guard_size as isize)) };
    }
    assert!(unsafe { libc::munmap(base as _, total_size) } >= 0);
}
