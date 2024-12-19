#![allow(dead_code)]
use std::io::{Error, ErrorKind};

fn io_error_kind_to_errno_constant(err: ErrorKind) -> libc::c_int {
    match err {
        ErrorKind::AlreadyExists => libc::EEXIST,
        _ => libc::EIO,
    }
}
pub fn io_error_to_errno_constant(err: &Error) -> libc::c_int {
    err.raw_os_error()
        .unwrap_or_else(|| io_error_kind_to_errno_constant(err.kind()))
}

pub(crate) fn close_handle_log_on_error(handle: super::RawFd) {
    if handle != super::INVALID_RAW_FD
        && unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) } == 0
    {
        log::warn!(
            "windows::CloseHandle({:?}) failed: {:?}",
            handle,
            std::io::Error::last_os_error()
        );
    }
}

pub(crate) fn close_socket_log_on_error(socket: super::RawSocket) {
    if socket != super::INVALID_RAW_SOCKET
        && unsafe { windows_sys::Win32::Networking::WinSock::closesocket(socket as _) } == 0
    {
        log::warn!(
            "windows::CloseHandle({:?}) failed: {:?}",
            socket,
            std::io::Error::last_os_error()
        );
    }
}
