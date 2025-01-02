// use super::UCtxRunCb;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        std::arch::global_asm!(include_str!("asm/x86_64_sysv_macho.S")); // TODO:
    } else {
        std::arch::global_asm!(include_str!("asm/x86_64_sysv_elf.S"));
    }
}

// pub(super) fn uctx_new(start_cb: UCtxRunCb, start_arg: *mut libc::c_void) -> UCtx {

// }
