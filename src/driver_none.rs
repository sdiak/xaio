use crate::Driver;
use crate::Sub;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct DriverNone {
    name: &'static str,
}

impl Default for DriverNone {
    fn default() -> Self {
        Self { name: "DriverNone" }
    }
}

impl Driver for DriverNone {
    fn name(&self) -> &'static str {
        self.name
    }
    fn submit(&mut self, _sub: std::pin::Pin<&mut Sub>) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    fn cancel(&mut self, _sub: std::pin::Pin<&Sub>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(
        &mut self,
        _timeout: Option<std::time::Duration>,
        _ready_list: &mut crate::SubList,
    ) -> std::io::Result<i32> {
        Err(Error::from(ErrorKind::Unsupported))
    }
}
