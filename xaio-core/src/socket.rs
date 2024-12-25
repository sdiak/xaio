use std::rc::Rc;
cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        use std::os::fd::AsRawFd;
    } else if #[cfg(target_family = "windows")] {
        std::os::windows::io::AsRawSocket;
    }
}

use crate::sys::RawSd;
pub use socket2::{Domain, Protocol, Type};

#[derive(Clone, Debug)]
pub struct Socket(Rc<Inner>);

cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        impl AsRawFd for Socket {
            fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
                self.0.socket.as_raw_fd()
            }
        }
    } else if #[cfg(target_family = "windows")] {
        impl AsRawSocket for Socket {
            fn as_raw_socket(&self) -> std::os::windows::io::RawSocket {
                todo!()
            }
        }
    }
}

#[derive(Debug)]
struct Inner {
    /// The socket
    socket: socket2::Socket,
    /// The identifier of the driver that owns this socket
    owner_id: usize,
    /// Driver specific data
    owner_data: usize,
}
