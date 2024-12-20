use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        pub type RawDirFd = libc::c_int;
        mod unix;
        pub use unix::*;
    } else if #[cfg(target_family = "windows")] {
        pub type RawDirFd = std::os::windows::raw::HANDLE;
        mod windows;
        pub use windows::*;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Options: u32 {
        /// Read access
        const READ       = 1u32 << 0;
        /// Write access
        const WRITE      = 1u32 << 1;
        /// Append mode (implies `Options::WRITE`)
        const APPEND     = 1u32 << 2;
        /// Create or open when already exists (`Options::WRITE` or `Options::APPEND` **must** be set)
        const CREATE     = 1u32 << 3;
        /// Create only when non existing (`Options::WRITE` or `Options::APPEND` **must** be set ; overrides `Options::CREATE`)
        const CREATE_NEW = 1u32 << 4;
        /// Truncates an existing file (`Options::WRITE` or `Options::APPEND` **must** be set)
        const TRUNCATE   = 1u32 << 5;
    }
}
impl Options {
    pub(crate) fn check(&self) -> Result<()> {
        if self.is_empty() {
            log::warn!("Empty options");
            Err(Error::from(ErrorKind::InvalidInput))
        } else {
            Ok(())
        }
    }
}
impl Default for Options {
    fn default() -> Self {
        Self::READ
    }
}

pub struct Dir {
    pub(crate) handle: RawDirFd,
}

impl Dir {
    pub fn open<P: AsRef<Path>>(path: P, options: Options) -> Result<Dir> {
        options.check()?;
        Err(Error::from(ErrorKind::Unsupported)) // TODO:
    }
    pub fn path(&self) -> Option<&PathBuf> {
        todo!()
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
