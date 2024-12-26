use std::{
    ffi::CStr,
    io::{Error, Result},
    sync::LazyLock,
};

use windows_sys::Win32::Foundation::HANDLE;

pub type RawSd = std::os::windows::raw::SOCKET;
pub type RawFd = std::os::windows::raw::HANDLE;

pub const INVALID_RAW_SD: RawSd = -1 as _;
pub const INVALID_RAW_FD: RawFd = usize::MAX as _;

#[inline(always)]
pub const fn raw_sd_is_valid(sd: RawSd) -> bool {
    sd != INVALID_RAW_SD
}
#[inline(always)]
pub const fn raw_fd_is_valid(sd: RawFd) -> bool {
    sd != INVALID_RAW_FD
}

pub mod ioutils;

pub mod event;

pub mod iocp;

pub mod statx;

#[derive(Debug, Clone, Copy)]
struct WindowsLateBinding {
    ntdll: Library,
    nt_create_wait_completion_packet: NtCreateWaitCompletionPacket,
    nt_associate_wait_completion_packet: NtAssociateWaitCompletionPacket,
}
impl Default for WindowsLateBinding {
    fn default() -> Self {
        let ntdll =
            Library::new("ntdll.dll").expect("Unrecoverable error while loading \"ntdll.dll\"");
        Self {
            ntdll,
            nt_create_wait_completion_packet: ntdll
                .get_proc_address::<NtCreateWaitCompletionPacket>(c"NtCreateWaitCompletionPacket")
                .expect("Needs ntdll::NtCreateWaitCompletionPacket"),
            nt_associate_wait_completion_packet: ntdll
                .get_proc_address::<NtAssociateWaitCompletionPacket>(
                    c"NtAssociateWaitCompletionPacket",
                )
                .expect("Needs ntdll::NtAssociateWaitCompletionPacket"),
        }
    }
}

static NTDLL: LazyLock<WindowsLateBinding> = LazyLock::new(WindowsLateBinding::default);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NtStatus {
    bits: u32,
    // https://joyasystems.com/list-of-ntstatus-codes
}
impl std::fmt::Debug for NtStatus {
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
    WaitCompletionPacketHandle: *mut libc::c_void,
    IoCompletionHandle: HANDLE,
    TargetObjectHandle: HANDLE,
    KeyContext: *mut libc::c_void,
    ApcContext: *mut libc::c_void,
    IoStatus: NtStatus,
    IoStatusInformation: usize,
    AlreadySignaled: *mut windows_sys::Win32::Foundation::BOOLEAN,
) -> NtStatus;

type NtCreateWaitCompletionPacket = extern "stdcall" fn(
    WaitCompletionPacketHandle: *mut HANDLE,
    DesiredAccess: u32,
    ObjectAttributes: *mut libc::c_void,
) -> NtStatus;

#[link(name = "kernel32")]
#[no_mangle]
extern "stdcall" {
    // fn GetLastError() -> u32;
    fn LoadLibraryExW(
        lpLibFileName: *const u16,
        hFile: *const libc::c_void,
        dwFlags: u32,
    ) -> *const libc::c_void;
    fn GetProcAddress(hModule: *const libc::c_void, lpProcName: *const u8) -> *const libc::c_void;
}

#[derive(Debug, Clone, Copy)]
struct Library {
    handle: *const libc::c_void,
}
unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Library {
    fn new(name: &str) -> Result<Self> {
        let handle = unsafe {
            LoadLibraryExW(
                name.encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
                std::ptr::null(),
                0x800,
            )
        };
        if handle.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(Self { handle })
        }
    }

    fn get_proc_address<F>(&self, proc_name: &CStr) -> Result<F>
    where
        F: Sized,
    {
        assert!(
            std::mem::size_of::<F>() == std::mem::size_of::<*const libc::c_void>(),
            "assert_foo_equals_bar"
        );
        let hproc = unsafe { GetProcAddress(self.handle, proc_name.as_ptr() as _) };
        if hproc.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(unsafe { std::mem::transmute_copy::<*const libc::c_void, F>(&hproc) })
        }
    }
}

pub fn last_os_error() -> i32 {
    unsafe { windows_sys::Win32::Foundation::GetLastError() as i32 }
}
