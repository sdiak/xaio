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
#[cfg(target_family = "unix")]
pub use unix::*;

#[cfg(target_family = "windows")]
mod windows;
#[cfg(target_family = "windows")]
pub use windows::*;

pub mod poll;

#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "linux", target_os = "freebsd")
))]
#[inline(always)]
pub fn get_current_thread_id() -> usize {
    use std::arch::asm;
    let mut id = 0usize;
    unsafe {
        asm!(
            "mov {id}, fs:0",
            id = out(reg) id,
        );
    }
    id as _
}
#[cfg(not(all(
    target_arch = "x86_64",
    any(target_os = "linux", target_os = "freebsd")
)))]
#[inline(always)]
pub fn get_current_thread_id() -> std::thread::ThreadId {
    std::thread::current().id()
}
