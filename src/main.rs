use std::fmt::Debug;
use std::io::Error;

use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows_sys::Win32::{
    Foundation::{BOOLEAN, GENERIC_ALL, HANDLE, INVALID_HANDLE_VALUE},
    System::IO::CreateIoCompletionPort,
};
use xaio::sys::Event;
// #[link(name = "kernel32")]
// #[link(name = "user32")]
// extern "stdcall" {
//     pub fn NtAssociateWaitCompletionPacket(
//         IoCompletionHandle: HANDLE,
//         TargetObjectHandle: HANDLE,
//         KeyContext: *mut libc::c_void,
//         ApcContext: *mut libc::c_void,
//         IoStatus: NTSTATUS,
//         IoStatusInformation: usize,
//         AlreadySignaled: *mut BOOLEAN,
//     ) -> NTSTATUS;
// }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NtStatus {
    bits: u32,
    // https://joyasystems.com/list-of-ntstatus-codes
}
impl Debug for NtStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("0x{:X}", self.bits))
    }
}
impl NtStatus {
    pub fn is_success(&self) -> bool {
        self.bits < 0x80000000u32
    }
    pub fn is_information(&self) -> bool {
        0x40000000u32 <= self.bits && self.bits < 0x80000000u32
    }
    pub fn is_warning(&self) -> bool {
        0x80000000u32 <= self.bits && self.bits < 0xC0000000u32
    }
    pub fn is_error(&self) -> bool {
        0xC0000000u32 <= self.bits
    }
}
type NtAssociateWaitCompletionPacket = extern "stdcall" fn(
    WaitCompletionPacketHandle: HANDLE,
    IoCompletionHandle: HANDLE,
    TargetObjectHandle: HANDLE,
    KeyContext: *mut libc::c_void,
    ApcContext: *mut libc::c_void,
    IoStatus: NtStatus,
    IoStatusInformation: usize,
    AlreadySignaled: *mut BOOLEAN,
) -> NtStatus;

type NtCreateWaitCompletionPacket = extern "stdcall" fn(
    WaitCompletionPacketHandle: *mut HANDLE,
    DesiredAccess: u32,
    ObjectAttributes: *mut libc::c_void,
) -> NtStatus;

pub fn main() {
    let ev = Event::new().unwrap();

    let ntdll = unsafe { LoadLibraryA(c"ntdll.dll".as_ptr() as _) };
    assert!(!ntdll.is_null());

    let nt_associate_wait_completion_packet =
        unsafe { GetProcAddress(ntdll, c"NtAssociateWaitCompletionPacket".as_ptr() as _) }.unwrap();
    let nt_associate_wait_completion_packet: NtAssociateWaitCompletionPacket =
        unsafe { std::mem::transmute(nt_associate_wait_completion_packet) };

    let nt_create_wait_completion_packet =
        unsafe { GetProcAddress(ntdll, c"NtCreateWaitCompletionPacket".as_ptr() as _) }.unwrap();
    let nt_create_wait_completion_packet: NtCreateWaitCompletionPacket =
        unsafe { std::mem::transmute(nt_associate_wait_completion_packet) };

    let iocp = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 0) };
    assert!(iocp != INVALID_HANDLE_VALUE);

    let mut completion_paquet: HANDLE = INVALID_HANDLE_VALUE;
    let r = unsafe {
        nt_create_wait_completion_packet(
            &mut completion_paquet,
            GENERIC_ALL as _,
            std::ptr::null_mut() as _,
        )
    };
    println!(
        "nt_create_wait_completion_packet => {:?} (succ:{}, info:{}, is_warning:{}, is_err:{})",
        r,
        r.is_success(),
        r.is_success(),
        r.is_warning(),
        r.is_error()
    );

    let r = unsafe {
        nt_associate_wait_completion_packet(
            completion_paquet,
            iocp,
            ev.native_handle(),
            std::ptr::null_mut() as _,
            std::ptr::null_mut() as _,
            NtStatus { bits: 0 },
            2 as _,
            std::ptr::null_mut() as _,
        )
    };
    println!(
        "nt_associate_wait_completion_packet => {:?} (succ:{}, info:{}, is_warning:{}, is_err:{})",
        r,
        r.is_success(),
        r.is_success(),
        r.is_warning(),
        r.is_error()
    );
    if r.is_error() {
        let e = Error::last_os_error();
        println!(" - {}", e);
    }

    windows_close_handle_log_on_error(iocp);
}

fn windows_close_handle_log_on_error(handle: windows_sys::Win32::Foundation::HANDLE) {
    if handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
        && unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) } == 0
    {
        log::warn!(
            "windows::CloseHandle({:?}) failed: {:?}",
            handle,
            std::io::Error::last_os_error()
        );
    }
}
