use std::io::{Error, ErrorKind};

// https://github.com/rust-lang/rust/issues/84277
const OTHER: i32 = libc::EIO as _;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Status {
    inner: i32,
}

impl Status {
    pub const PENDING: i32 = i32::MIN;

    #[inline(always)]
    pub const fn new(status: i32) -> Self {
        debug_assert!(status != Status::PENDING);
        Self { inner: status }
    }
    #[cfg(target_family = "unix")]
    pub fn last_os_error() -> Self {
        Self {
            inner: -crate::sys::last_os_error(),
        }
    }
    #[inline(always)]
    pub const fn pending() -> Self {
        Self {
            inner: Status::PENDING,
        }
    }

    #[inline(always)]
    pub const fn value(&self) -> i32 {
        self.inner
    }
    #[inline(always)]
    pub const fn is_pending(&self) -> bool {
        self.inner == Status::PENDING
    }
    #[inline(always)]
    pub const fn is_success(&self) -> bool {
        self.inner >= 0
    }
    #[inline(always)]
    pub const fn is_error(&self) -> bool {
        Status::PENDING < self.inner && self.inner < 0
    }
}

impl From<ErrorKind> for Status {
    fn from(value: ErrorKind) -> Self {
        let status = match value {
            ErrorKind::NotFound => libc::ENOENT,
            ErrorKind::PermissionDenied => libc::EPERM,
            ErrorKind::ConnectionRefused => libc::ECONNREFUSED,
            ErrorKind::ConnectionReset => libc::ECONNRESET,
            ErrorKind::HostUnreachable => libc::EHOSTUNREACH,
            ErrorKind::NetworkUnreachable => libc::ENETUNREACH,
            ErrorKind::ConnectionAborted => libc::ECONNABORTED,
            ErrorKind::NotConnected => libc::ENOTCONN,
            ErrorKind::AddrInUse => libc::EADDRINUSE,
            ErrorKind::AddrNotAvailable => libc::EADDRNOTAVAIL,
            ErrorKind::NetworkDown => libc::ENETDOWN,
            ErrorKind::BrokenPipe => libc::EPIPE,
            ErrorKind::AlreadyExists => libc::EEXIST,
            ErrorKind::WouldBlock => libc::EAGAIN,
            ErrorKind::NotADirectory => libc::ENOTDIR,
            ErrorKind::IsADirectory => libc::EISDIR,
            #[cfg(not(target_os = "aix"))]
            ErrorKind::DirectoryNotEmpty => libc::ENOTEMPTY,
            ErrorKind::ReadOnlyFilesystem => libc::EROFS,
            #[cfg(not(target_os = "windows"))]
            ErrorKind::StaleNetworkFileHandle => libc::ESTALE,
            ErrorKind::InvalidInput => libc::EINVAL,
            ErrorKind::TimedOut => libc::ETIMEDOUT,
            ErrorKind::StorageFull => libc::ENOSPC,
            ErrorKind::NotSeekable => libc::ESPIPE,
            ErrorKind::FileTooLarge => libc::EFBIG,
            ErrorKind::ResourceBusy => libc::EBUSY,
            ErrorKind::ExecutableFileBusy => libc::ETXTBSY,
            ErrorKind::Deadlock => libc::EDEADLK,
            ErrorKind::TooManyLinks => libc::EMLINK,
            ErrorKind::ArgumentListTooLong => libc::E2BIG,
            ErrorKind::Interrupted => libc::EINTR,
            ErrorKind::Unsupported => libc::ENOSYS,
            ErrorKind::OutOfMemory => libc::ENOMEM,
            _ => OTHER,
        };
        Status::new(-status)
    }
}

#[cfg(target_family = "unix")]
impl From<Error> for Status {
    fn from(value: Error) -> Self {
        if let Some(status) = value.raw_os_error() {
            Status::new(-status)
        } else {
            value.kind().into()
        }
    }
}

#[cfg(target_family = "unix")]
impl From<Status> for Error {
    fn from(val: Status) -> Self {
        if val.is_error() {
            Error::from_raw_os_error(-val.value())
        } else {
            Error::from(ErrorKind::Other)
        }
    }
}

// #[cfg(target_family = "windows")]
// fn raw_last_error_to_errno(win_error: i32) -> i32 {
//     log::warn!("TODO:");
//     win_error
// }
#[cfg(target_family = "windows")]
impl From<Error> for Status {
    fn from(value: Error) -> Self {
        value.kind().into()
    }
}
#[cfg(target_family = "windows")]
impl From<Status> for Error {
    fn from(val: Status) -> Self {
        let val = val.value();
        log::warn!("TODO:");
        Error::from(ErrorKind::Other)
        // if request::PENDING < val && val < 0 {
        //     Error::from_raw_os_error(-val)
        // } else {
        //     Error::from(ErrorKind::Other)
        // }
    }
}
