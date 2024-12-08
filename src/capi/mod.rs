#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

// cbindgen --config cbindgen.toml --crate xaio
mod driver;

#[cfg_attr(target_os = "linux", path = "driver_epoll_linux.rs")]
#[cfg_attr(not(target_os = "linux"), path = "driver_epoll_unsupported.rs")]
mod rawpoll;

/// A thread-local completion port
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct xcp_s {
    prv__thread_id: usize,
    // prv__driver:
    prv__id: u32,
    prv__refcount: u32,
    prv__now: i64,
}

#[repr(C)]
#[derive(Debug)]
pub struct xaio_s {
    prv__cp: *mut xcp_s,
    prv__status: i32,
    prv__flags_and_op_code: u32,
    prv__next: std::sync::atomic::AtomicUsize,
}

extern "C" {
    /// Creates a new completion port bound to the current thread.
    ///
    /// # Arguments
    ///   - `pport` `*pport` receives a new completion port address or `NULL` on error.
    ///   - `opt_driver` driver to **move** to the port or `NULL` to use the default driver.
    ///
    /// # Returns
    ///   -  `0` on success
    ///   -  `-EINVAL` when `pport == NULL`
    ///   -  `-ENOMEM` when the system is out of memory
    #[must_use]
    pub fn xcp_new(pport: *mut *mut xcp_s) -> i32;
}
