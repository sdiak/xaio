use std::{
    io::{Error, ErrorKind, Result},
    ptr::NonNull,
};

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::Request;

// TODO: replace pin with addr

#[cfg(not(target_os = "windows"))]
type Fd = libc::c_int;
#[cfg(target_os = "windows")]
type Fd = usize;

pub(crate) struct FdMap {
    entries: FxHashMap<Fd, Entry>,
}

struct Entry {
    reader: Option<NonNull<Request>>,
    writer: Option<NonNull<Request>>,
}

impl FdMap {
    pub(crate) fn new(capacity: usize) -> Result<Self> {
        match std::panic::catch_unwind(|| {
            FxHashMap::<Fd, Entry>::with_capacity_and_hasher(capacity, FxBuildHasher)
        }) {
            Ok(entries) => Ok(Self { entries }),
            Err(_) => Err(Error::from(ErrorKind::OutOfMemory)),
        }
    }
    pub(crate) fn update(
        &mut self,
        fd: Fd,
        reader: Option<NonNull<Request>>,
        writer: Option<NonNull<Request>>,
    ) -> std::io::Result<()> {
        if let Some(entry) = self.entries.get_mut(&fd) {
            // Check for a single reader and a single writer
            if (entry.reader.is_some() && reader.is_some())
                || (entry.writer.is_some() && writer.is_some())
            {
                return Err(std::io::Error::from(std::io::ErrorKind::ResourceBusy));
            }
            entry.reader = reader;
            entry.writer = writer;
            // Drop the entry if there are no more watchers
            if entry.reader.is_none() && entry.writer.is_none() {
                self.entries.remove(&fd);
            }
        } else if reader.is_some() || writer.is_some() {
            self.entries.try_reserve(1)?;
            self.entries
                .insert(fd, Entry { reader, writer })
                .expect("Memory is reserved");
        }
        Ok(())
    }
}
