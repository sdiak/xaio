use std::rc::Rc;
cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        use std::os::fd::AsRawFd;
    } else if #[cfg(target_family = "windows")] {
        std::os::windows::io::AsRawSocket;
    }
}

use crate::sys::RawSd;

#[derive(Clone, Debug)]
pub struct Socket(Rc<Inner>);

cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        impl AsRawFd for Socket {
            fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
                self.0.sd as _
            }
        }
    } else if #[cfg(target_family = "windows")] {
        impl AsRawSocket for Socket {
            fn as_raw_socket(&self) -> std::os::windows::io::RawSocket {
                self.0.sd as _
            }
        }
    }
}

#[derive(Debug)]
struct Inner {
    /// The raw socket descriptor
    sd: RawSd,
    /// The identifier of the driver that owns this socket
    owner_id: usize,
}
impl Drop for Inner {
    fn drop(&mut self) {
        crate::sys::ioutils::close_log_on_error(self.sd);
    }
}
