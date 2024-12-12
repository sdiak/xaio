#[cfg(not(target_os = "windows"))]
use std::os::fd::RawFd as Inner;
#[cfg(target_os = "windows")]
use std::os::windows::raw::SOCKET as Inner;

use std::io::{Error, ErrorKind, Result};

#[derive(Debug, Copy, PartialEq, Eq, Clone, PartialOrd, Ord, Hash)]
pub struct RawSocketFd {
    pub(crate) inner: Inner,
}

impl RawSocketFd {
    pub fn new(fd: Inner) -> Self {
        Self { inner: fd }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn invalid() -> Self {
        Self {
            inner: -1i32 as Inner,
        }
    }
    #[cfg(target_os = "windows")]
    pub fn invalid() -> Self {
        Self {
            inner: windows_sys::Win32::Networking::WinSock::INVALID_SOCKET as _,
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_valid(&self) -> bool {
        self.inner >= (0 as Inner)
    }
    #[cfg(target_os = "windows")]
    pub fn is_valid(&self) -> bool {
        self.inner != (windows_sys::Win32::Networking::WinSock::INVALID_SOCKET as _)
    }
}

#[cfg(target_family = "windows")]
pub fn socketpair(
    domain: socket2::Domain,
    typ: socket2::Type,
    protocol: Option<socket2::Protocol>,
) -> Result<(socket2::Socket, socket2::Socket)> {
    match domain {
        socket2::Domain::IPV4 => {}
        _ => return Err(Error::from(ErrorKind::InvalidInput)),
    }
    match typ {
        socket2::Type::STREAM => {}
        _ => return Err(Error::from(ErrorKind::InvalidInput)),
    }
    if let Some(p) = protocol {
        match p {
            socket2::Protocol::TCP => {}
            _ => return Err(Error::from(ErrorKind::InvalidInput)),
        }
    }
    let listener = socket2::Socket::new(domain, typ, protocol)?;
    listener.set_reuse_address(true)?;
    let addr: socket2::SockAddr =
        std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 0).into();
    listener.bind(&addr)?;
    let aaddr = listener.local_addr()?;

    listener.listen(1)?;

    let b = socket2::Socket::new(domain, typ, protocol)?;
    b.connect(&aaddr)?;

    let (a, _) = listener.accept()?;
    if a.peer_addr()? == b.local_addr()? && a.local_addr()? == b.peer_addr()? {
        Ok((a, b))
    } else {
        Err(Error::from(ErrorKind::Other))
    }
}
