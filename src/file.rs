use crate::Request;
use std::fs::File;
use std::io::Result;
use std::mem::ManuallyDrop;
#[cfg(target_family = "unix")]
use std::os::fd::FromRawFd;
#[cfg(target_family = "unix")]
use std::os::unix::fs::FileExt;
#[cfg(target_family = "windows")]
use std::os::windows::io::FromRawHandle;

fn _file_io_sync<F>(req: &mut Request, f: F) -> i32
where
    F: FnOnce(&File, &mut [u8], u64) -> Result<()>,
{
    let todo = unsafe { req.op.file_io.todo } as usize;
    let offset = unsafe { req.op.file_io.offset };
    #[cfg(target_family = "unix")]
    let file = ManuallyDrop::new(unsafe { File::from_raw_fd(req.op.file_io.fd) });
    #[cfg(target_family = "windows")]
    let file = ManuallyDrop::new(unsafe { File::from_raw_handle(req.op.file_io.handle) });

    let buffer = unsafe { std::slice::from_raw_parts_mut::<u8>(req.op.file_io.buffer, todo) };

    match f(&file, buffer, offset) {
        Ok(_) => todo as i32,
        Err(err) => -crate::utils::io_error_to_errno_constant(&err),
    }
}

#[cfg(target_family = "unix")]
#[inline(always)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> Result<()> {
    file.read_exact_at(buf, offset)
}
#[cfg(target_family = "unix")]
#[inline(always)]
fn write_all_at(file: &File, buf: &mut [u8], offset: u64) -> Result<()> {
    file.write_all_at(buf, offset)
}

#[cfg(target_family = "windows")]
fn read_or_write_exact_at(file: &File, buf: &mut [u8], offset: u64, read: bool) -> Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let mut ovlp: OVERLAPPED = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let todo = buf.len() as u32;
    let mut done = 0u32;
    while done < todo {
        ovlp.Anonymous.Anonymous.Offset = ((offset + done as u64) & 0xFFFFFFFFu64) as _;
        ovlp.Anonymous.Anonymous.OffsetHigh = ((offset + done as u64) >> 32) as _;
        let mut just_did = 0u32;
        if (read
            && unsafe {
                ReadFile(
                    file.as_raw_handle(),
                    buf.as_mut_ptr().offset(done as _),
                    buf.len() as u32 - done,
                    &mut just_did as _,
                    &mut ovlp as _,
                ) == 0
            })
            || (!read
                && unsafe {
                    WriteFile(
                        file.as_raw_handle(),
                        buf.as_mut_ptr().offset(done as _),
                        buf.len() as u32 - done,
                        &mut just_did as _,
                        &mut ovlp as _,
                    ) == 0
                })
        {
            return Err(std::io::Error::last_os_error());
        }
        done += just_did;
    }
    Ok(())
}

#[cfg(target_family = "windows")]
#[inline(always)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> Result<()> {
    read_or_write_exact_at(file, buf, offset, true)
}
#[cfg(target_family = "windows")]
#[inline(always)]
fn write_all_at(file: &File, buf: &mut [u8], offset: u64) -> Result<()> {
    read_or_write_exact_at(file, buf, offset, false)
}

pub fn file_io_read_sync(req: &mut Request) -> i32 {
    _file_io_sync(req, read_exact_at)
}
pub fn file_io_write_sync(req: &mut Request) -> i32 {
    _file_io_sync(req, write_all_at)
}
// pub(crate) fn file_io(req: &mut Request, ready: &mut ReadyList) -> i32 {
//     let status = req.status.load(Ordering::Relaxed);
//     if status != request::PENDING {
//         return status; // Canceled or Timedout
//     }
//     let status = match req.opcode_raw() {
//         request::OP_FILE_READ => _file_io_sync(req, File::read_exact_at),
//         request::OP_FILE_WRITE => {
//             _file_io_sync(req, |file, buf, offset| file.write_all_at(buf, offset))
//         }
//         _ => {
//             panic!("Unknown operation type : {:?}", req.opcode());
//         }
//     };
//     assert!(status != request::PENDING);
//     status
// }
