use std::fmt::Debug;
use std::io::Error;

// use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows_sys::Win32::{
    Foundation::{BOOLEAN, GENERIC_ALL, HANDLE, HMODULE, INVALID_HANDLE_VALUE},
    System::IO::CreateIoCompletionPort,
};
use xaio::sys::Event;

#[link(name = "kernel32")]
#[no_mangle]
extern "stdcall" {
    fn GetLastError() -> u32;
    fn LoadLibraryExW(
        lpLibFileName: *const u16,
        hFile: *const libc::c_void,
        dwFlags: u32,
    ) -> *const libc::c_void;
    // fn FreeLibrary(hLibModule: *const c_void) -> i32;
    fn GetProcAddress(hModule: *const libc::c_void, lpProcName: *const u8) -> *const libc::c_void;
}

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

struct WinLib {
    handle: HMODULE,
}

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

type FnMessageBox = extern "stdcall" fn(
    hWnd: *const libc::c_void,
    lpText: *const u16,
    lpCaption: *const u16,
    uType: u32,
) -> i32;

trait IntoNullTerminatedU16 {
    fn to_nullterminated_u16(&self) -> Vec<u16>;
}

impl IntoNullTerminatedU16 for str {
    fn to_nullterminated_u16(&self) -> Vec<u16> {
        self.encode_utf16().chain(Some(0)).collect()
    }
}

pub fn main() {
    let user32dll = unsafe {
        LoadLibraryExW(
            "user32.dll".to_nullterminated_u16().as_ptr(),
            std::ptr::null(),
            0x800,
        )
    };
    assert!(!user32dll.is_null());
    let fn_message_box = unsafe { GetProcAddress(user32dll, "MessageBoxW\0".as_ptr() as _) };
    println!("{fn_message_box:?}");
    let fn_message_box = unsafe { std::mem::transmute::<_, FnMessageBox>(fn_message_box) };
    let r = fn_message_box(
        std::ptr::null(),
        "Hello, Rust!".to_nullterminated_u16().as_ptr(),
        "MessageBox".to_nullterminated_u16().as_ptr(),
        0,
    );
    println!("fn_message_box => {r}");

    let ev = Event::new().unwrap();

    let ntdll = unsafe {
        LoadLibraryExW(
            "ntdll.dll".to_nullterminated_u16().as_ptr(),
            std::ptr::null(),
            0x800,
        )
    };
    assert!(!ntdll.is_null());

    let nt_associate_wait_completion_packet =
        unsafe { GetProcAddress(ntdll, c"NtAssociateWaitCompletionPacket".as_ptr() as _) };
    let nt_associate_wait_completion_packet = unsafe {
        std::mem::transmute::<_, NtAssociateWaitCompletionPacket>(
            nt_associate_wait_completion_packet,
        )
    };

    let nt_create_wait_completion_packet =
        unsafe { GetProcAddress(ntdll, c"NtCreateWaitCompletionPacket".as_ptr() as _) };
    let nt_create_wait_completion_packet = unsafe {
        std::mem::transmute::<_, NtCreateWaitCompletionPacket>(nt_associate_wait_completion_packet)
    };

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
