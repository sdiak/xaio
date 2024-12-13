use std::io::{Error, ErrorKind, Read, Write};
use std::mem::zeroed;
use std::os::windows::io::{AsRawSocket, AsSocket};
use std::{fmt::Debug, mem::MaybeUninit};

use windows_sys::Win32::System::IO::{GetQueuedCompletionStatusEx, OVERLAPPED};
// use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows_sys::Win32::Networking::WinSock::{WSARecv, WSASend, WSABUF};
use windows_sys::Win32::{
    Foundation::{BOOLEAN, GENERIC_ALL, HANDLE, HMODULE, INVALID_HANDLE_VALUE},
    System::IO::{CreateIoCompletionPort, OVERLAPPED_ENTRY},
};
use xaio::sys::Event;

#[derive(Clone, Copy)]
pub struct OverlappedEntry(OVERLAPPED_ENTRY);

impl Debug for OverlappedEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "OverlappedEntry(internal:{:?}, status:{:?}, key:{:?}, overlapped:{:?})",
            self.0.Internal,
            self.0.dwNumberOfBytesTransferred,
            self.0.lpCompletionKey,
            self.0.lpOverlapped
        ))
    }
}

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
    // let r = fn_message_box(
    //     std::ptr::null(),
    //     "Hello, Rust!".to_nullterminated_u16().as_ptr(),
    //     "MessageBox".to_nullterminated_u16().as_ptr(),
    //     0,
    // );
    // println!("fn_message_box => {r}");

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
        unsafe { GetProcAddress(ntdll, "NtAssociateWaitCompletionPacket\0".as_ptr() as _) };
    let nt_associate_wait_completion_packet = unsafe {
        std::mem::transmute::<_, NtAssociateWaitCompletionPacket>(
            nt_associate_wait_completion_packet,
        )
    };

    let nt_create_wait_completion_packet =
        unsafe { GetProcAddress(ntdll, "NtCreateWaitCompletionPacket\0".as_ptr() as _) };
    let nt_create_wait_completion_packet = unsafe {
        std::mem::transmute::<_, NtCreateWaitCompletionPacket>(nt_create_wait_completion_packet)
    };

    let iocp = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 0) };
    assert!(iocp != INVALID_HANDLE_VALUE);

    let mut completion_paquet: HANDLE = std::ptr::null_mut();
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
        r.is_information(),
        r.is_warning(),
        r.is_error()
    );

    let r = unsafe {
        nt_associate_wait_completion_packet(
            completion_paquet,
            iocp,
            ev.get_native_handle(),
            std::ptr::null_mut() as _,
            0x40000000u32 as _,
            NtStatus { bits: 0 },
            2 as _,
            std::ptr::null_mut() as _,
        )
    };
    println!(
        "nt_associate_wait_completion_packet => {:?} (succ:{}, info:{}, is_warning:{}, is_err:{})",
        r,
        r.is_success(),
        r.is_information(),
        r.is_warning(),
        r.is_error()
    );
    if r.is_error() {
        let e = Error::last_os_error();
        println!(" - {}", e);
    }

    let mut buffer: [OverlappedEntry; 5] =
        [unsafe { std::mem::MaybeUninit::zeroed().assume_init() }; 5];
    let mut nentries: u32 = 0;
    let r = unsafe {
        GetQueuedCompletionStatusEx(
            iocp,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut nentries as _,
            100 as _,
            0,
        )
    };
    assert!(r != 0 || Error::last_os_error().kind() == ErrorKind::TimedOut);

    ev.notify().unwrap();

    let r = unsafe {
        GetQueuedCompletionStatusEx(
            iocp,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut nentries as _,
            1000 as _,
            0,
        )
    };
    assert!(r != 0 && nentries == 1);
    println!("message: {:?}", buffer[0usize]);

    ev.notify().unwrap();
    let r = unsafe {
        GetQueuedCompletionStatusEx(
            iocp,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut nentries as _,
            100 as _,
            0,
        )
    };
    assert!(r != 0 || Error::last_os_error().kind() == ErrorKind::TimedOut);

    unsafe {
        nt_associate_wait_completion_packet(
            completion_paquet,
            iocp,
            ev.get_native_handle(),
            std::ptr::null_mut() as _,
            0xCAFEBABEu32 as _,
            NtStatus { bits: 0 },
            42 as _,
            std::ptr::null_mut() as _,
        );
    }
    let r = unsafe {
        GetQueuedCompletionStatusEx(
            iocp,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut nentries as _,
            1000 as _,
            0,
        )
    };
    assert!(r != 0 && nentries == 1);
    println!("message: {:?}", buffer[0usize]);

    // windows_close_handle_log_on_error(iocp);

    let (mut a, mut b) = xaio::socketpair(socket2::Type::STREAM, None).unwrap();
    println!("({a:?}, {b:?})");

    let mut wasmsg = [' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8];
    let wsabuf = WSABUF {
        len: 4,
        buf: &mut wasmsg as _,
    };
    let mut wasmsg2 = [' ' as u8, ' ' as u8, ' ' as u8, ' ' as u8];
    let wsabuf2 = WSABUF {
        len: 4,
        buf: &mut wasmsg2 as _,
    };
    let mut lpnumberofbytessent: u32 = 0;
    let mut lpnumberofbytesrecvd: u32 = 0;
    let mut ovlp1: OVERLAPPED = unsafe { MaybeUninit::zeroed().assume_init() };
    let r = unsafe {
        GetQueuedCompletionStatusEx(
            iocp,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut nentries as _,
            100 as _,
            0,
        )
    };
    let iocp_socket = unsafe {
        CreateIoCompletionPort(
            a.as_raw_socket() as usize as HANDLE,
            iocp,
            a.as_raw_socket() as _, // Use the handle as key to ease dealing with completions
            0,
        )
    };
    assert!(iocp_socket != INVALID_HANDLE_VALUE);

    assert!(r != 0 || Error::last_os_error().kind() == ErrorKind::TimedOut);
    let mut lpflags: u32 = 0;
    let status = unsafe {
        WSARecv(
            a.as_raw_socket() as _,
            &wsabuf,
            1,
            &mut lpnumberofbytesrecvd,
            &mut lpflags as _,
            &mut ovlp1 as _,
            None,
        )
        // WSASend(
        //     a.as_raw_socket() as usize,
        //     &wsabuf,
        //     1,
        //     &mut lpnumberofbytessent as _,
        //     0,
        //     &mut ovlp as _,
        //     None,
        // )
    };
    let err = unsafe { GetLastError() };
    println!(
        "Status={status}, lpflags={lpflags} err={}({err})",
        Error::last_os_error()
    );
    let mut ovlp2: OVERLAPPED = unsafe { MaybeUninit::zeroed().assume_init() };
    let status = unsafe {
        WSARecv(
            a.as_raw_socket() as _,
            &wsabuf2,
            1,
            &mut lpnumberofbytesrecvd,
            &mut lpflags as _,
            &mut ovlp2 as _,
            None,
        )
        // WSASend(
        //     a.as_raw_socket() as usize,
        //     &wsabuf,
        //     1,
        //     &mut lpnumberofbytessent as _,
        //     0,
        //     &mut ovlp as _,
        //     None,
        // )
    };
    let err = unsafe { GetLastError() };
    println!(
        "Status={status}, lpflags={lpflags} err={}({err})",
        Error::last_os_error()
    );
    // assert!(status<0 && err.kind())
    let r = unsafe {
        GetQueuedCompletionStatusEx(
            iocp,
            buffer.as_mut_ptr() as _,
            buffer.len() as _,
            &mut nentries as _,
            100 as _,
            0,
        )
    };
    assert!(r != 0 || Error::last_os_error().kind() == ErrorKind::TimedOut);

    let msg = ['a' as u8, 'b' as u8, 'c' as u8, 'd' as u8];
    println!(
        "A wrote {} bytes: {:?}",
        a.write(&msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    let mut msg = ['a' as u8, 'a' as u8, 'a' as u8, 'a' as u8];
    println!(
        "B read {} bytes: {:?}",
        b.read(&mut msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    let msg = ['O' as u8, 'k' as u8, '!' as u8];
    println!(
        "B wrote {} bytes: {:?}",
        b.write(&msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );
    let msg = ['D' as u8, 'o' as u8, 'n' as u8, 'e' as u8];
    println!(
        "B wrote {} bytes: {:?}",
        b.write(&msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    // let r = unsafe {
    //     GetQueuedCompletionStatusEx(
    //         iocp,
    //         buffer.as_mut_ptr() as _,
    //         buffer.len() as _,
    //         &mut nentries as _,
    //         100 as _,
    //         0,
    //     )
    // };
    // assert!(r == 1);
    // for c in 0..r {
    //     println!(
    //         "Completion: {:?} socker={} overlapped-addr={:?}",
    //         buffer[c as usize],
    //         a.as_raw_socket(),
    //         &ovlp1 as *const OVERLAPPED as *const libc::c_void
    //     )
    // }
    // println!(
    //     "A read {} bytes: {:?} (r={r})",
    //     '?',
    //     std::str::from_utf8(&wasmsg).unwrap()
    // );

    let mut remaining = 2;
    while remaining > 0 {
        let r = unsafe {
            GetQueuedCompletionStatusEx(
                iocp,
                buffer.as_mut_ptr() as _,
                buffer.len() as _,
                &mut nentries as _,
                1000 as _,
                0,
            )
        };
        assert!(r == 1);
        println!(
            "Socket={}, ov1={:?}, ov2={:?}",
            a.as_raw_socket(),
            &ovlp1 as *const OVERLAPPED as *const libc::c_void,
            &ovlp2 as *const OVERLAPPED as *const libc::c_void
        );
        for c in 0..nentries {
            println!("Completion[{c}]: {:?}", buffer[c as usize],);
            remaining -= 1;
            let n = buffer[c as usize].0.dwNumberOfBytesTransferred as usize;
            let buf = if buffer[c as usize].0.lpOverlapped == &mut ovlp1 {
                std::str::from_utf8(&wasmsg).unwrap()
            } else {
                std::str::from_utf8(&wasmsg2).unwrap()
            };

            println!("A read {} bytes: {:?} (r={r})", n, &buf[..n]);
        }
    }
    // let mut msg = ['a' as u8, 'a' as u8, 'a' as u8, 'a' as u8];
    // println!(
    //     "A read {} bytes: {:?}",
    //     a.read(&mut msg).unwrap(),
    //     std::str::from_utf8(&msg).unwrap()
    // );

    println!(
        "a:{:?}, b:{:?}",
        a.as_socket().as_raw_socket(),
        b.as_socket().as_raw_socket()
    );
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
