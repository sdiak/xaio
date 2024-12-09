use crate::Driver;
use crate::Sub;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct DriverURing {
    ringfd: libc::c_int,
}

// https://github.com/axboe/liburing/wiki/io_uring-and-networking-in-2023
// https://github.com/axboe/liburing/blob/master/examples/io_uring-udp.c

// impl Default for DriverURing {
//     fn default() -> Self {
//         Self { epfd: -1 as _ }
//     }
// }
/*impl Driver for DriverURing {
    fn name(&self) -> &'static str {
        "DriverURing"
    }
    fn submit(&mut self, _sub: std::pin::Pin<&mut Sub>) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    fn cancel(&mut self, _sub: std::pin::Pin<&Sub>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait_ms(
        &mut self,
        _timeout: Option<std::time::Duration>,
        _ready_list: &mut crate::SubList,
    ) -> std::io::Result<i32> {
        Err(Error::from(ErrorKind::Unsupported))
    }
}
 */
