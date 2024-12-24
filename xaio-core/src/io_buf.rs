use std::{
    alloc::Layout,
    io::{Error, ErrorKind, Result},
};

const BUFFER_ALIGN: usize = 16;

pub struct IoBuf {
    group_id: u16, // u16_MAX for simple buffer
    buffer_id: u16,
    len: u32,
    addr: *mut u8,
}

impl IoBuf {
    pub fn new(len: usize) -> Result<IoBuf> {
        if len >= i32::MAX as usize {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        if let Ok(layout) = Layout::from_size_align(len, BUFFER_ALIGN) {
            let addr = unsafe { std::alloc::alloc(layout) };
            if addr.is_null() {
                return Err(Error::from(ErrorKind::OutOfMemory));
            }
            Ok(Self {
                group_id: u16::MAX,
                buffer_id: u16::MAX,
                len: len as _,
                addr,
            })
        } else {
            Err(Error::from(ErrorKind::InvalidInput))
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len as _
    }
    pub(crate) unsafe fn data(&self) -> *const u8 {
        self.addr
    }
    pub(crate) unsafe fn data_mut(&mut self) -> *mut u8 {
        self.addr
    }

    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.addr, self.len as _) }
    }
    pub fn as_slice_mut<'a>(&'a mut self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.addr, self.len as _) }
    }
}

impl Drop for IoBuf {
    fn drop(&mut self) {
        if self.group_id == u16::MAX {
            let layout = Layout::from_size_align(self.len(), BUFFER_ALIGN)
                .expect("Validated by IoBuf::new()");
            unsafe {
                std::alloc::dealloc(self.addr, layout);
            };
        } else {
            todo!()
        }
    }
}
