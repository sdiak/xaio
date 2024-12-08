use std::pin::Pin;

use rustc_hash::FxHashMap;

use crate::Sub;

#[cfg(not(target_os = "windows"))]
type Fd = libc::c_int;
#[cfg(target_os = "windows")]
type Fd = libc::usize;

pub(crate) struct FdMap<'a> {
    readers: FxHashMap<Fd, Pin<&'a Sub>>,
    writers: FxHashMap<Fd, Pin<&'a Sub>>,
}

impl<'a> FdMap<'a> {
    pub(crate) fn add_s_reader(&'a mut self, fd: Fd, reader: Pin<&'a Sub>) -> std::io::Result<()> {
        if self.readers.contains_key(&fd) {
            Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy))
        } else if self.readers.try_reserve(1).is_ok() {
            self.readers.insert(fd, reader);
            Ok(())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory))
        }
    }
    pub(crate) fn remove_s_reader(&'a mut self, fd: Fd) -> Option<Pin<&'a Sub>> {
        self.readers.remove(&fd)
    }

    pub(crate) fn add_s_writer(&'a mut self, fd: Fd, writer: Pin<&'a Sub>) -> std::io::Result<()> {
        if self.writers.contains_key(&fd) {
            Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy))
        } else if self.writers.try_reserve(1).is_ok() {
            self.writers.insert(fd, writer);
            Ok(())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory))
        }
    }
    pub(crate) fn remove_s_writer(&'a mut self, fd: Fd) -> Option<Pin<&'a Sub>> {
        self.writers.remove(&fd)
    }
}
