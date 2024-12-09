use crate::{DriverConfig, DriverIFace, Request};
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct DriverNone {
    name: &'static str,
    config: DriverConfig,
}

impl Default for DriverNone {
    fn default() -> Self {
        Self {
            name: "DriverNone",
            config: DriverConfig::default(),
        }
    }
}

impl DriverIFace for DriverNone {
    fn config(&self) -> &DriverConfig {
        &self.config
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn submit(&mut self, _sub: std::pin::Pin<&mut Request>) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    fn cancel(&mut self, _sub: std::pin::Pin<&Request>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(
        &mut self,
        _timeout: Option<std::time::Duration>,
        _ready_list: &mut crate::RequestList,
    ) -> std::io::Result<i32> {
        Err(Error::from(ErrorKind::Unsupported))
    }
}
