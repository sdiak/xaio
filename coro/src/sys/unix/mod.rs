#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
pub mod asm;

pub type UCtxRunCb = unsafe extern "C" fn(*mut libc::c_void);
pub struct UCtx {
    stack_pointer: *mut libc::c_void,
}
