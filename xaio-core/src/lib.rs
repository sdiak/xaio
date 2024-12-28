#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::{
    cell::Cell,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::Arc,
};

// pub mod completion_queue;
mod io_req;
// pub mod io_req_fifo;
// pub mod io_req_lifo;
pub mod sys;
pub use io_req::*;

mod op_code;
pub use op_code::*;

mod completion_port;
pub use completion_port::*;

pub mod collection;

pub mod io_driver;

mod poll_flags;
pub use poll_flags::*;

mod io_buf;
pub use io_buf::*;

mod socket;
pub use socket::*;

mod status;
pub use status::*;

pub type PhantomUnsync = std::marker::PhantomData<std::cell::Cell<()>>;
pub type PhantomUnsend = std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>;

mod r#async;
pub use r#async::*;

fn catch_enomem<C, T>(constructor: C) -> std::io::Result<T>
where
    C: FnOnce() -> T + std::panic::UnwindSafe,
{
    std::panic::catch_unwind(constructor)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}

#[macro_export]
macro_rules! pin_mut {
    ($var:ident) => {
        let mut $var = $var;
        #[allow(unused_mut)]
        let mut $var = unsafe { Pin::new_unchecked(&mut $var) };
    };
}

pub trait Unpark {
    fn unpark(&self);
}
// #[derive(Clone)]
pub struct Unparker {
    target: Box<dyn Unpark>,
}
impl Unparker {
    pub fn new<U: Unpark + std::panic::UnwindSafe + Send + Sync + Clone + 'static>(
        target: U,
    ) -> Self {
        Self {
            target: Box::new(target),
        }
    }
    pub fn unpark(&self) {
        self.target.unpark();
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub type Socket = socket2::Socket;

#[repr(transparent)]
pub struct Uniq<T: Sized>(std::ptr::NonNull<T>);

impl<T: Sized> Uniq<T> {
    pub const LAYOUT: std::alloc::Layout = unsafe {
        std::alloc::Layout::from_size_align_unchecked(
            std::mem::size_of::<T>(),
            std::mem::align_of::<T>(),
        )
    };
    pub fn new(value: T) -> Option<Self> {
        let ptr = unsafe { std::alloc::alloc(Self::LAYOUT) } as *mut T;
        if !ptr.is_null() {
            unsafe { ptr.write(value) };
            Some(Self(unsafe { NonNull::new_unchecked(ptr) }))
        } else {
            None
        }
    }
    pub fn as_ptr(&mut self) -> *mut T {
        self.0.as_ptr()
    }
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
    pub fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
    pub unsafe fn from_raw(raw: *mut T) -> Uniq<T> {
        Self(unsafe { NonNull::new_unchecked(raw) })
    }
    pub unsafe fn into_raw(self) -> *mut T {
        let raw = self.0.as_ptr();
        std::mem::forget(self);
        raw
    }
}
impl<T: Sized> Deref for Uniq<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
impl<T: Sized> DerefMut for Uniq<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}
impl<T: Sized> Drop for Uniq<T> {
    fn drop(&mut self) {
        let ptr = self.0.as_ptr();
        unsafe {
            std::ptr::drop_in_place(ptr);
            std::alloc::dealloc(ptr as _, Self::LAYOUT);
        }
    }
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
