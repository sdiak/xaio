use std::pin::Pin;

use rustc_hash::FxHashMap;

use crate::Request;

// TODO: replace pin with addr

#[cfg(not(target_os = "windows"))]
type Fd = libc::c_int;
#[cfg(target_os = "windows")]
type Fd = usize;

pub(crate) struct FdMap<'a> {
    readers: FxHashMap<Fd, Pin<&'a Request>>,
    writers: FxHashMap<Fd, Pin<&'a Request>>,
}

impl<'a> FdMap<'a> {
    pub(crate) fn add_sequential_reader(
        &'a mut self,
        fd: Fd,
        reader: Pin<&'a Request>,
    ) -> std::io::Result<()> {
        if self.readers.contains_key(&fd) {
            Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy))
        } else if self.readers.try_reserve(1).is_ok() {
            self.readers.insert(fd, reader);
            Ok(())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory))
        }
    }
    pub(crate) fn remove_sequential_reader(&'a mut self, fd: Fd) -> Option<Pin<&'a Request>> {
        self.readers.remove(&fd)
    }

    pub(crate) fn add_sequential_writer(
        &'a mut self,
        fd: Fd,
        writer: Pin<&'a Request>,
    ) -> std::io::Result<()> {
        if self.writers.contains_key(&fd) {
            Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy))
        } else if self.writers.try_reserve(1).is_ok() {
            self.writers.insert(fd, writer);
            Ok(())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory))
        }
    }
    pub(crate) fn remove_sequential_writer(&'a mut self, fd: Fd) -> Option<Pin<&'a Request>> {
        self.writers.remove(&fd)
    }
}
