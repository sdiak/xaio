use crate::sys::OsSocketAddr;
use num::ToPrimitive;
use std::fmt::Debug;
use std::i32;
use std::io::{Error, ErrorKind, Result};
use std::mem::MaybeUninit;
use std::os::windows::io::RawSocket;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Networking::WinSock::{self, AcceptEx, WSABUF};
use windows_sys::Win32::Storage::FileSystem;
use windows_sys::Win32::Storage::FileSystem::{CreateFileA, ReadFile, WriteFile};
use windows_sys::Win32::System::IO::{
    GetQueuedCompletionStatusEx, PostQueuedCompletionStatus, OVERLAPPED, OVERLAPPED_ENTRY,
};
use windows_sys::Win32::{
    Foundation::{BOOLEAN, HANDLE},
    System::IO::CreateIoCompletionPort,
};

use super::event::Event;

const WAKE_TOKEN: usize = usize::MAX;

#[derive(Debug, Clone)]
pub struct IoCompletionPort {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    handle: HANDLE,
    bound_events: Mutex<Vec<BoundEvent>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        super::ioutils::close_handle_log_on_error(self.handle);
    }
}

#[derive(Debug, Clone)]
struct BoundEvent {
    event: Event,
    overlapped: *const OVERLAPPED_ENTRY,
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct RawAsyncFile(HANDLE);

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Overlapped(OVERLAPPED);
impl Overlapped {
    pub fn new() -> Self {
        unsafe { MaybeUninit::<Self>::zeroed().assume_init() }
    }

    #[inline(always)]
    pub fn status(&self) -> usize {
        self.0.Internal
    }

    #[inline(always)]
    pub fn bytes_transferred(&self) -> usize {
        self.0.InternalHigh
    }

    #[inline(always)]
    pub fn offset(&self) -> u64 {
        unsafe {
            (self.0.Anonymous.Anonymous.OffsetHigh as u64).wrapping_shl(32)
                + self.0.Anonymous.Anonymous.Offset as u64
        }
    }
    #[inline(always)]
    pub fn set_offset(&mut self, offset: u64) {
        self.0.Anonymous.Anonymous.Offset = (offset & 0xFFFFFFFFu64) as _;
        self.0.Anonymous.Anonymous.OffsetHigh = offset.wrapping_shr(32) as _;
    }

    #[inline]
    pub fn event(&self) -> Option<super::event::RawEvent> {
        let handle = self.0.hEvent;
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(super::event::RawEvent { handle })
        }
    }
}
impl Debug for Overlapped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlapped")
            .field("status", &self.status())
            .field("bytes_transferred", &self.bytes_transferred())
            .field("offset", &self.offset())
            .field("event", &self.event())
            .finish()
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct OverlappedEntry(OVERLAPPED_ENTRY);
impl OverlappedEntry {
    pub fn new() -> Self {
        unsafe { MaybeUninit::<Self>::zeroed().assume_init() }
    }

    #[inline(always)]
    pub fn token(&self) -> usize {
        self.0.lpCompletionKey
    }
    #[inline(always)]
    pub fn overlapped(&self) -> Option<NonNull<Overlapped>> {
        NonNull::<Overlapped>::new(self.0.lpOverlapped as *mut Overlapped)
    }
}
impl Debug for OverlappedEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ovlp: &dyn Debug = if let Some(o) = self.overlapped() {
            unsafe { o.as_ref() }
        } else {
            &self.overlapped()
        };
        f.debug_struct("OverlappedEntry")
            .field("token", &self.token())
            .field("overlapped", ovlp)
            .finish_non_exhaustive()
    }
}

impl IoCompletionPort {
    pub fn new(max_number_of_threads: u32) -> Result<Self> {
        let bound_events: Mutex<Vec<BoundEvent>> = Mutex::new(Vec::default());
        let handle = {
            let handle = unsafe {
                CreateIoCompletionPort(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null_mut(),
                    0,
                    max_number_of_threads as _,
                )
            };
            if handle.is_null() {
                return Err(Error::last_os_error());
            }
            handle
        };
        Ok(Self {
            inner: Arc::new(Inner {
                handle,
                bound_events,
            }),
        })
    }

    pub fn bind_event(&mut self, event: &Event) {
        let events = self.inner.bound_events.lock();
        let binding = &*super::NTDLL;

        // events.
        todo!();
    }

    pub fn get_native_handle(&self) -> HANDLE {
        self.inner.handle
    }

    fn push_completion_status(
        &self,
        len: u32,
        token: usize,
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        if unsafe { PostQueuedCompletionStatus(self.inner.handle, len, token, overlapped as _) }
            != 0
        {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
    pub fn wake(&self) -> Result<()> {
        self.push_completion_status(0, WAKE_TOKEN, std::ptr::null_mut())
    }

    pub fn wait(&self, events: &mut [OverlappedEntry], timeout_ms: i32) -> std::io::Result<i32> {
        let timeout_ms = if timeout_ms < 0 {
            0xFFFFFFFFu32
        } else {
            timeout_ms as u32
        };
        let capacity = events.len().to_i32().unwrap_or(i32::MAX);
        let mut nentries: u32 = 0;
        if (unsafe {
            GetQueuedCompletionStatusEx(
                self.inner.handle,
                events.as_mut_ptr() as _,
                capacity as _,
                &mut nentries as _,
                timeout_ms,
                0,
            )
        }) != 0
        {
            // Filter out wake events
            let mut i = 0usize;
            while i < nentries as usize {
                if events[i].token() == WAKE_TOKEN {
                    // remove by swapping out with the last one
                    // println!("Skip wake {i}");
                    events[i] = events[(nentries as usize) - 1];
                    nentries -= 1;
                } else {
                    i += 1;
                }
            }
            Ok(nentries as _)
        } else {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            match err {
                windows_sys::Win32::Foundation::WAIT_TIMEOUT => Ok(0),
                _ => Err(Error::from_raw_os_error(err as _)),
            }
        }
    }

    pub fn attach_file(&self, hfile: super::RawFd, token: usize) -> Result<()> {
        if token == WAKE_TOKEN {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        if unsafe { CreateIoCompletionPort(hfile, self.inner.handle, token, 0).is_null() } {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[inline(always)]
    pub fn attach_socket(&self, socket: super::RawSocket, token: usize) -> Result<()> {
        self.attach_file(socket as _, token)
    }

    pub fn read(
        &self,
        hfile: super::RawFd,
        buffer: &mut [u8],
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        let capacity = buffer.len().to_u32().unwrap_or(u32::MAX);
        if unsafe {
            ReadFile(
                hfile,
                buffer.as_mut_ptr(),
                capacity,
                std::ptr::null_mut(),
                overlapped as _,
            )
        } != 0
        {
            println!("TODO: read completed inline");
            Ok(())
        } else {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            match err {
                windows_sys::Win32::Foundation::ERROR_IO_PENDING => {
                    println!(" read pending");
                    Ok(())
                }
                windows_sys::Win32::Foundation::ERROR_INVALID_USER_BUFFER => {
                    Err(Error::from(ErrorKind::WouldBlock))
                }
                windows_sys::Win32::Foundation::ERROR_NOT_ENOUGH_MEMORY => {
                    Err(Error::from(ErrorKind::WouldBlock))
                }
                _ => Err(Error::from_raw_os_error(err as _)),
            }
        }
    }

    pub fn write(
        &self,
        hfile: super::RawFd,
        buffer: &[u8],
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        let capacity = buffer.len().to_u32().unwrap_or(u32::MAX);
        if unsafe {
            WriteFile(
                hfile,
                buffer.as_ptr(),
                capacity,
                std::ptr::null_mut(),
                overlapped as _,
            )
        } != 0
        {
            println!("TODO: write completed inline");
            Ok(())
        } else {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            match err {
                windows_sys::Win32::Foundation::ERROR_IO_PENDING => {
                    println!(" write pending");
                    Ok(())
                }
                windows_sys::Win32::Foundation::ERROR_INVALID_USER_BUFFER => {
                    Err(Error::from(ErrorKind::WouldBlock))
                }
                windows_sys::Win32::Foundation::ERROR_NOT_ENOUGH_MEMORY => {
                    Err(Error::from(ErrorKind::WouldBlock))
                }
                _ => Err(Error::from_raw_os_error(err as _)),
            }
        }
    }

    pub fn accept(&self, listen_socket: RawSocket, buf: *mut AcceptBuffer) -> Result<()> {
        let rruf = unsafe { &mut *buf };
        let mut bytesreceived: libc::c_ulong = 0;
        if unsafe {
            WinSock::AcceptEx(
                listen_socket as _,
                rruf.client as _,
                rruf.addresses_memory.as_mut_ptr() as _,
                0,
                (std::mem::size_of::<OsSocketAddr>() + 16) as _,
                (std::mem::size_of::<OsSocketAddr>() + 16) as _,
                &mut bytesreceived as _,
                &mut rruf.overlapped as *mut Overlapped as _,
            ) != 0
        } {
            println!("TODO: AcceptEx completed inline");
            Ok(())
        } else {
            let err = unsafe { WinSock::WSAGetLastError() };
            match err {
                WinSock::WSA_IO_PENDING => {
                    println!(" AcceptEx pending");
                    Ok(())
                }
                _ => Err(wsaerror_to_error(err)),
            }
        }
    }

    pub fn recvv(
        &self,
        socket: super::RawSocket,
        buffers: &mut [WSABUF],
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        if buffers.len() > libc::c_ulong::MAX as _ {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        let mut flags: libc::c_ulong = 0 as _;
        if unsafe {
            WinSock::WSARecv(
                socket as _,
                buffers.as_mut_ptr(),
                buffers.len() as _,
                std::ptr::null_mut(),
                &mut flags as _,
                overlapped as _,
                None,
            )
        } != WinSock::SOCKET_ERROR
        {
            println!("TODO: recv completed inline");
            Ok(())
        } else {
            let err = unsafe { WinSock::WSAGetLastError() };
            match err {
                WinSock::WSA_IO_PENDING => {
                    println!(" recv pending");
                    Ok(())
                }
                _ => Err(wsaerror_to_error(err)),
            }
        }
    }
    pub fn recv(
        &self,
        socket: super::RawSocket,
        buffer: &mut [u8],
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        let len = buffer.len().to_u32().unwrap_or(u32::MAX);
        let mut buffers = [WSABUF {
            len,
            buf: buffer.as_mut_ptr(),
        }; 1];
        self.recvv(socket, &mut buffers, overlapped)
    }
    pub fn sendv(
        &self,
        socket: super::RawSocket,
        buffers: &[WSABUF],
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        if buffers.len() > libc::c_ulong::MAX as _ {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        if unsafe {
            WinSock::WSASend(
                socket as _,
                buffers.as_ptr(),
                buffers.len() as _,
                std::ptr::null_mut(),
                0,
                overlapped as _,
                None,
            )
        } != WinSock::SOCKET_ERROR
        {
            println!("TODO: send completed inline");
            // self.push_completion_status(buffers.len() as _, 9876, overlapped)
            Ok(())
        } else {
            let err = unsafe { WinSock::WSAGetLastError() };
            match err {
                WinSock::WSA_IO_PENDING => {
                    println!(" send pending");
                    Ok(())
                }
                _ => Err(wsaerror_to_error(err)),
            }
        }
    }
    pub fn send(
        &self,
        socket: super::RawSocket,
        buffer: &[u8],
        overlapped: *mut Overlapped,
    ) -> Result<()> {
        let len = buffer.len().to_u32().unwrap_or(u32::MAX);
        let buffers = [WSABUF {
            len,
            buf: buffer.as_ptr() as _,
        }; 1];
        self.sendv(socket, &buffers, overlapped)
    }

    pub fn open(
        &self,
        path: *const i8,
        access: u32,
        sharemode: u32,
        create_disposition: u32,
    ) -> Result<RawAsyncFile> {
        let hfile = unsafe {
            CreateFileA(
                path as _,
                access,
                sharemode,
                std::ptr::null_mut(),
                create_disposition,
                FileSystem::FILE_ATTRIBUTE_NORMAL | FileSystem::FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        if hfile == INVALID_HANDLE_VALUE {
            Err(Error::last_os_error())
        } else {
            Ok(RawAsyncFile(hfile))
        }
    }
}

pub fn wsaerror_to_error(err: WinSock::WSA_ERROR) -> Error {
    match err {
        WinSock::WSAECONNABORTED => Error::from(ErrorKind::ConnectionAborted),
        WinSock::WSAECONNRESET => Error::from(ErrorKind::ConnectionReset),
        WinSock::WSAECONNREFUSED => Error::from(ErrorKind::ConnectionRefused),
        WinSock::WSAENOTCONN => Error::from(ErrorKind::NotConnected),
        WinSock::WSAEFAULT => Error::from(ErrorKind::InvalidInput),
        _ => {
            log::warn!("TODO: wsaerror_to_error({})", err);
            Error::from(ErrorKind::Other)
        }
    }
}

struct DD<const S: usize> {
    d: [u8; S],
}

#[repr(C)]
pub struct AcceptBuffer {
    client: RawSocket,
    len: libc::c_ulong,
    overlapped: Overlapped,
    /// @see [GetAcceptExSockaddrs](https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nf-mswsock-getacceptexsockaddrs) to parse
    addresses_memory: [u8; std::mem::size_of::<OsSocketAddr>() + 2 * 16],
}
impl AcceptBuffer {
    pub fn new(accept_socket: RawSocket) -> Self {
        Self {
            client: accept_socket,
            len: 0,
            overlapped: Overlapped::new(),
            addresses_memory: unsafe { std::mem::zeroed() },
        }
    }
    pub fn addresses(&self) -> (Option<&OsSocketAddr>, Option<&OsSocketAddr>) {
        let mut local: *mut OsSocketAddr = std::ptr::null_mut();
        let mut remote: *mut OsSocketAddr = std::ptr::null_mut();
        let plocal: *mut *mut OsSocketAddr = &mut local;
        let premote: *mut *mut OsSocketAddr = &mut remote;
        unsafe {
            WinSock::GetAcceptExSockaddrs(
                &self.addresses_memory as *const u8 as *mut libc::c_void,
                self.len,
                (std::mem::size_of::<OsSocketAddr>() + 16) as _,
                (std::mem::size_of::<OsSocketAddr>() + 16) as _,
                plocal as _,
                std::mem::size_of::<OsSocketAddr>() as _,
                premote as _,
                std::mem::size_of::<OsSocketAddr>() as _,
            )
        };
        let local: Option<&OsSocketAddr> = if local.is_null() {
            None
        } else {
            Some(unsafe { &*local })
        };
        let remote: Option<&OsSocketAddr> = if remote.is_null() {
            None
        } else {
            Some(unsafe { &*remote })
        };
        (local, remote)
    }
}

#[cfg(test)]
mod tests {
    use core::str;
    use std::{
        fs::File,
        io::Write,
        os::windows::io::{AsRawHandle, AsRawSocket, AsSocket},
    };

    use super::*;

    #[test]
    fn it_works() {
        let mut evbuf = [OverlappedEntry::new(); 4];
        let iocp = IoCompletionPort::new(0).unwrap();
        iocp.wake().unwrap();
        iocp.wake().unwrap();
        iocp.wake().unwrap();
        println!("\n - wait => {:?}", iocp.wait(&mut evbuf, 0));

        let mut to_read = tempfile::NamedTempFile::new().unwrap();
        let mut to_write = tempfile::NamedTempFile::new().unwrap();

        println!(
            "To-read:{:?}, to-write:{:?}",
            to_read.path(),
            to_write.path()
        );
        write!(to_read.as_file(), "This is my test data").unwrap();
        write!(to_write.as_file(), "-").unwrap();
        crate::sys::ioutils::close_handle_log_on_error(to_read.as_file().as_raw_handle());
        crate::sys::ioutils::close_handle_log_on_error(to_write.as_file().as_raw_handle());

        // let file =
        // std::fs::File::open("C:\\Users\\diakites\\Downloads\\cm-support-admin.tar.gz").unwrap();
        for _ in 0..2 {
            let hfile = iocp
                .open(
                    to_read.path().as_os_str().as_encoded_bytes().as_ptr() as _,
                    windows_sys::Win32::Foundation::GENERIC_READ,
                    0,
                    FileSystem::OPEN_EXISTING,
                )
                .unwrap();
            //file.as_raw_handle();
            iocp.attach_file(hfile.0, 1).unwrap();
            let mut buffer = [0u8; 256];
            let mut overlapped = Overlapped::new();
            iocp.read(hfile.0, &mut buffer, &mut overlapped as _)
                .unwrap();

            let r = iocp.wait(&mut evbuf, 0).unwrap();
            println!("\n - wait(r) => {:?}", r);
            for i in 0..r as usize {
                println!("  =>  {:?}", evbuf[i]);
                if let Some(ovlp) = evbuf[i].overlapped() {
                    let ovlp = unsafe { ovlp.as_ref() };
                    println!(
                        "  =====> {:?}",
                        String::from_utf8_lossy(&buffer[..ovlp.bytes_transferred()])
                    );
                }
            }
            crate::sys::ioutils::close_handle_log_on_error(hfile.0);

            let hfile = iocp
                .open(
                    to_write.path().as_os_str().as_encoded_bytes().as_ptr() as _,
                    windows_sys::Win32::Foundation::GENERIC_WRITE,
                    0,
                    FileSystem::CREATE_ALWAYS,
                )
                .unwrap();
            iocp.attach_file(hfile.0, 42).unwrap();
            let mut overlapped = Overlapped::new();
            iocp.write(hfile.0, &buffer[..16], &mut overlapped as _)
                .unwrap();
            let r = iocp.wait(&mut evbuf, 100).unwrap();
            println!("\n - wait(w) => {:?}", r);
            for i in 0..r {
                println!("  =>  {:?}", evbuf[i as usize]);
            }
            crate::sys::ioutils::close_handle_log_on_error(hfile.0);
        }
    }

    #[test]
    fn test_socket() {
        use socket2::{Domain, Socket, Type};
        use std::io::{Read, Write};
        use std::net::{SocketAddr, TcpStream};
        let mut evbuf = [OverlappedEntry::new(); 4];
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
        let iocp = IoCompletionPort::new(0).unwrap();
        let address: SocketAddr = "127.0.0.1:8282".parse().unwrap();
        let address = address.into();
        socket.bind(&address).unwrap();
        socket.listen(128).unwrap();

        // TODO: IOCPAccept
        iocp.attach_socket(socket.as_raw_socket(), 1234).unwrap();
        let client = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
        let mut abuf = AcceptBuffer::new(client.as_raw_socket());
        iocp.attach_socket(client.as_raw_socket(), 4321).unwrap();
        iocp.accept(socket.as_raw_socket(), &mut abuf as _).unwrap();
        let r = iocp.wait(&mut evbuf, -1).unwrap();
        println!("\n - wait(accept) => {:?}", r);
        for i in 0..r as usize {
            println!("  =>  {:?}", evbuf[i]);
            assert!(evbuf[i].token() == 1234);
            if let Some(ovlp) = evbuf[i].overlapped() {
                let ovlp = unsafe { ovlp.as_ref() };
                println!("  =====> {:?}", ovlp.bytes_transferred());
            }
        }
        println!("Addresses: {:?}", abuf.addresses());

        let raw_socket = abuf.client;
        let mut buf = [b' '; 256];
        // let (client, addr) = socket.accept().unwrap();
        // let raw_socket = client.as_socket().as_raw_socket();
        // iocp.attach_socket(raw_socket, 4242).unwrap();
        // println!("New client {:?}", addr);
        // let mut buf = [b' '; 256];

        // let mut client: TcpStream = client.into();
        // client.read(&mut buf);
        // println!("Got {}", str::from_utf8(&buf).unwrap());
        // client.write("World".as_bytes()).unwrap();
        // client.shutdown(std::net::Shutdown::Both).unwrap();
        let mut overlapped = Overlapped::new();
        iocp.recv(raw_socket, &mut buf, &mut overlapped).unwrap();
        let r = iocp.wait(&mut evbuf, 3000).unwrap();
        println!("\n - wait(recv) => {:?}", r);
        for i in 0..r as usize {
            println!("  =>  {:?}", evbuf[i]);
            if let Some(ovlp) = evbuf[i].overlapped() {
                let ovlp = unsafe { ovlp.as_ref() };
                println!(
                    "  =====> {:?}",
                    String::from_utf8_lossy(&buf[..ovlp.bytes_transferred()])
                );
            }
        }
        iocp.send(raw_socket, "World!".as_bytes(), &mut overlapped)
            .unwrap();
        let r = iocp.wait(&mut evbuf, 3000).unwrap();
        println!("\n - wait(send) => {:?}", r);
        for i in 0..r as usize {
            println!("  =>  {:?}", evbuf[i]);
            if let Some(ovlp) = evbuf[i].overlapped() {
                let ovlp = unsafe { ovlp.as_ref() };
                println!("  =====> Sent: {:?}", ovlp.bytes_transferred());
            }
        }
        println!("storage: {}", std::mem::size_of::<OsSocketAddr>());
        // client.shutdown(std::net::Shutdown::Write).unwrap();
        // iocp.recv(raw_socket, &mut buf, &mut overlapped).unwrap();
        // let r = iocp.wait(&mut evbuf, 3000).unwrap();
        // println!("\n - wait(r) => {:?}", r);
        // for i in 0..r as usize {
        //     println!("  =>  {:?}", evbuf[i]);
        //     if let Some(ovlp) = evbuf[i].overlapped() {
        //         let ovlp = unsafe { ovlp.as_ref() };
        //         println!(
        //             "  =====> {:?}",
        //             String::from_utf8_lossy(&buf[..ovlp.bytes_transferred()])
        //         );
        //     }
        // }
        // std::mem::forget(client);
        // unsafe { WinSock::closesocket(raw_socket as _) };
    }
}
