use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

pub mod sys;

fn catch_enomem<C, T>(constructor: C) -> std::io::Result<T>
where
    C: FnOnce() -> T + std::panic::UnwindSafe,
{
    std::panic::catch_unwind(constructor)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}

cfg_if::cfg_if! {
    if #[cfg(not(target_os = "windows"))] {
        use libc::{sockaddr, sockaddr_in as sockaddr_in, sockaddr_in6, sockaddr_storage, AF_INET, AF_INET6, sockaddr_un, AF_UNIX};
    } else {
        use windows_sys::Win32::Networking::WinSock::{SOCKADDR as sockaddr, SOCKADDR_IN as sockaddr_in, SOCKADDR_IN6 as sockaddr_in6, SOCKADDR_STORAGE as sockaddr_storage, AF_INET, AF_INET6};
        #[cfg(feature = "win-af-unix")]
        use windows_sys::Win32::Networking::WinSock::{SOCKADDR_UN as sockaddr_un, AF_UNIX};
        #[cfg(not(feature = "win-af-unix"))]
        const AF_UNIX: u16 = 1;
    }
}

// use windo
// use winapi::shared::{
//     ws2def::{AF_INET, AF_INET6, SOCKADDR as sockaddr, SOCKADDR_IN as sockaddr_in},
//     ws2ipdef::SOCKADDR_IN6_LH as sockaddr_in6,
// };

#[repr(u16)]
pub enum AddrFamily {
    INET = AF_INET,
    INET6 = AF_INET6,
    UNIX = AF_UNIX,
}

#[repr(C)]
pub union SockAddr {
    sa: sockaddr,
    sa4: sockaddr_in,
    sa6: sockaddr_in6,
    #[cfg(feature = "win-af-unix")]
    sau: sockaddr_un,
    sas: sockaddr_storage,
}

impl SockAddr {
    pub fn family(&self) -> Option<AddrFamily> {
        match unsafe { self.sa.sa_family } {
            AF_INET => Some(AddrFamily::INET),
            AF_INET6 => Some(AddrFamily::INET6),
            AF_UNIX => Some(AddrFamily::UNIX),
            _ => None,
        }
    }
    pub fn to_addr(&self) -> Option<SocketAddr> {
        match unsafe { self.sa.sa_family } {
            AF_INET => {
                #[cfg(not(target_os = "windows"))]
                todo!();
                #[cfg(target_os = "windows")]
                let ip = unsafe { self.sa4.sin_addr.S_un.S_addr };
                Some(SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::from(u32::from_be(ip)),
                    u16::from_be(unsafe { self.sa4.sin_port }),
                )))
            }
            AF_INET6 => {
                #[cfg(not(target_os = "windows"))]
                todo!();
                #[cfg(target_os = "windows")]
                let (ip, scope) = (unsafe { self.sa6.sin6_addr.u.Byte }, unsafe {
                    self.sa6.Anonymous.sin6_scope_id
                });
                Some(SocketAddr::V6(SocketAddrV6::new(
                    Ipv6Addr::from(u128::from_be_bytes(ip)),
                    u16::from_be(unsafe { self.sa6.sin6_port }),
                    unsafe { self.sa6.sin6_flowinfo },
                    scope,
                )))
            }
            _ => None,
        }
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
