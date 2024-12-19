use num::ToPrimitive;
use std::fmt::Debug;
use std::i32;
use std::io::{Error, ErrorKind, Result};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
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

    pub fn wake(&self) -> Result<()> {
        if unsafe {
            PostQueuedCompletionStatus(self.inner.handle, 0, WAKE_TOKEN, std::ptr::null_mut())
        } != 0
        {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
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

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, os::windows::io::AsRawHandle};

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
}
