use nix::sys::epoll::Epoll;
use std::os::fd::RawFd;
use std::os::raw::c_void;
use std::pin::Pin;

use super::driver::{
    xdriver_class_s, xdriver_is_supported, xdriver_params_s, xdriver_s, XDRIVER_FLAG_CLOSE_ON_EXEC,
};

use super::{xaio_s, xcp_s};

#[repr(C)]
struct EPoll {
    sup: xdriver_s,
    fd: RawFd,
}

extern "C" fn xepoll_default_params(params: &mut xdriver_params_s) -> &mut xdriver_params_s {
    params.flags |= XDRIVER_FLAG_CLOSE_ON_EXEC;
    params.max_number_of_fd_hint = 1024;
    params
}
extern "C" fn xepoll_open(thiz: &mut c_void, port: &xcp_s) -> i32 {
    let thiz: &mut EPoll = unsafe { &mut *(std::ptr::from_mut(thiz) as *mut EPoll) };
    let flags = if (thiz.sup.params.flags & XDRIVER_FLAG_CLOSE_ON_EXEC) != 0u32 {
        libc::EPOLL_CLOEXEC
    } else {
        0 as libc::c_int
    };
    thiz.fd = unsafe { libc::epoll_create1(flags) };
    if thiz.fd < 0 {
        -std::io::Error::last_os_error().raw_os_error().unwrap() // safety: constructed by last_os_error()
    } else {
        0
    }
}
extern "C" fn xepoll_close(thiz: &mut c_void, port: &xcp_s) {
    let thiz: &mut EPoll = unsafe { &mut *(std::ptr::from_mut(thiz) as *mut EPoll) };
    if thiz.fd > -1 && unsafe { libc::close(thiz.fd) } < 0 {
        log::warn!(
            "xepoll_close: failed closing the epoll file descriptor {}: {:?}",
            thiz.fd,
            std::io::Error::last_os_error()
        );
    }
    thiz.fd = -1;
}

extern "C" fn xepoll_get_native_handle(thiz: &c_void, phandle: &mut usize) -> i32 {
    let thiz: &EPoll = unsafe { &*(std::ptr::from_ref(thiz) as *const EPoll) };
    *phandle = thiz.fd as usize;
    -1
}

extern "C" fn xepoll_wait(thiz: &mut c_void, timeout: i32, events_sink: &mut c_void) -> i32 {
    -1
}
extern "C" fn xepoll_submit(thiz: &mut c_void, op: Pin<&mut xaio_s>) -> i32 {
    -1
}
extern "C" fn xepoll_cancel(thiz: &mut c_void, op: Pin<&xaio_s>) -> i32 {
    -1
}

const XDRIVER_CLASS_EPOLL: xdriver_class_s = xdriver_class_s {
    name: c"epoll".as_ptr(),
    flags: 0 as _,
    instance_align: 0 as _,
    instance_size: 0 as _,
    is_supported: xdriver_is_supported,
    default_params: xepoll_default_params,
    open: xepoll_open,
    close: xepoll_close,
    get_native_handle: xepoll_get_native_handle,
    wait: xepoll_wait,
    submit: xepoll_submit,
    cancel: xepoll_cancel,
};
