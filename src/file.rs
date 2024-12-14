use crate::{request, ReadyList, Request};
use std::fs::File;
use std::io::{ErrorKind, Result};
use std::mem::ManuallyDrop;
#[cfg(target_family = "unix")]
use std::os::unix::fs::FileExt;
use std::sync::atomic::Ordering;

fn _file_io_sync<F>(req: &mut Request, f: F) -> i32
where
    F: FnOnce(&File, &mut [u8], u64) -> Result<()>,
{
    use std::os::fd::FromRawFd;
    let todo = unsafe { req.op.file_io.todo } as usize;
    let offset = unsafe { req.op.file_io.offset };
    let file = ManuallyDrop::new(unsafe { File::from_raw_fd(req.op.file_io.fd) });
    let buffer = unsafe { std::slice::from_raw_parts_mut::<u8>(req.op.file_io.buffer, todo) };

    match f(&file, buffer, offset) {
        Ok(_) => todo as i32,
        Err(err) => -crate::utils::io_error_to_errno_constant(&err),
    }
}

pub(crate) fn file_io(req: &mut Request, ready: &mut ReadyList) {
    if req.concurrent_status.load(Ordering::Relaxed) != request::PENDING {
        return; // Canceled or Timedout
    }
    let status = match req.opcode_raw() {
        request::OP_FILE_READ => _file_io_sync(req, File::read_exact_at),
        request::OP_FILE_WRITE => {
            _file_io_sync(req, |file, buf, offset| file.write_all_at(buf, offset))
        }
        _ => {
            panic!("Unknown operation type : {:?}", req.opcode());
        }
    };
    assert!(status != request::PENDING);
    loop {
        match req.concurrent_status.compare_exchange_weak(
            request::PENDING,
            status,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Err(curent) if curent == request::PENDING => {}
            _ => {
                // Canceled or Timedout
                break;
            }
        }
    }
}
