use crate::{
    DriverConfig, DriverFlags, DriverHandle, DriverIFace, Request, AN_INVALID_DRIVER_HANDLE,
};
use std::io::{Error, ErrorKind, Result};
use std::ptr::NonNull;

const DRIVER_NAME: &str = "None";

#[derive(Debug)]
pub struct DriverNone {
    name: &'static str,
    config: DriverConfig,
    // TODO: create a pipe
}

impl Default for DriverNone {
    fn default() -> Self {
        Self {
            name: DRIVER_NAME,
            config: DriverConfig::default(),
        }
    }
}

impl DriverNone {
    pub fn new(config: &DriverConfig, name: Option<&'static str>) -> Result<Self> {
        let mut real_config: DriverConfig =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        real_config.max_number_of_fd_hint = num::clamp(config.max_number_of_fd_hint, 1, 1000000);
        real_config.flags = DriverFlags::CLOSE_ON_EXEC.bits();
        let name = name.unwrap_or(DRIVER_NAME);
        Ok(DriverNone {
            name,
            config: real_config,
        })
    }
}
impl DriverIFace for DriverNone {
    fn config(&self) -> &DriverConfig {
        &self.config
    }
    fn name(&self) -> &'static str {
        self.name
    }
    unsafe fn submit(&mut self, _req: NonNull<Request>) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    unsafe fn cancel(&mut self, _req: NonNull<Request>) -> std::io::Result<()> {
        Err(Error::from(ErrorKind::NotFound))
    }
    fn wait(
        &mut self,
        _ready_list: &mut crate::RequestList,
        timeout_ms: i32,
    ) -> std::io::Result<i32> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    fn wake(&self) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }
    unsafe fn get_native_handle(&self) -> DriverHandle {
        AN_INVALID_DRIVER_HANDLE
    }
}
