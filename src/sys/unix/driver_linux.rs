use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::LazyLock;
use std::io::{Error, Result};
use super::{EPoll, Event};
use uring_sys2;
use num;


pub static PROBE: LazyLock<Probe> = LazyLock::new(Probe::new);


#[derive(Debug)]
pub(crate) struct Driver {
    ring: URing,
    epoll: EPoll, // -1 when polling using ring
    waker: Event, // TODO: register
    config: crate::capi::xconfig_s,
}


impl Driver {
    pub(crate) fn new(config_hints: &crate::capi::xconfig_s) -> Result<Self> {
        let mut config = *config_hints;
        config.submission_queue_depth = num::clamp(config.submission_queue_depth, 16, 4096);
        if config.completion_queue_depth < config.submission_queue_depth * 2 {
            config.completion_queue_depth = config.submission_queue_depth * 2;
        } else {
            config.completion_queue_depth = num::clamp(config.completion_queue_depth, 16, 4096);
        }
        config.flags = config.flags & (crate::capi::XCONFIG_FLAG_FAST_POLL | crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC | crate::capi::XCONFIG_FLAG_ATTACH_SINGLE_ISSUER | crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC );
        let waker = Event::new()?;
        let mut ring = URing::invalid();
        let probe = &*PROBE;
        let mut epoll = EPoll::invalid();
        if probe.has_io_uring() {
            ring = URing::new(&mut config, probe)?;
            if (config.flags & crate::capi::XCONFIG_FLAG_FAST_POLL) != 0 {
                epoll = EPoll::new((config.flags & crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC) != 0)?;
            }
        } else {
            epoll = EPoll::new((config.flags & crate::capi::XCONFIG_FLAG_CLOSE_ON_EXEC) != 0)?;
        }
        Ok(Self { ring, epoll, waker, config})
    }
}


#[derive(Debug)]
struct URing {
    ring: uring_sys2::io_uring,
    features: u32,
}
impl Drop for URing {
    fn drop(&mut self) {
        if self.ring.ring_fd >= 0 {
            unsafe { uring_sys2::io_uring_queue_exit(&mut self.ring as _) };
        }
    }
}
impl URing {
    fn invalid() -> Self {
        let mut ring = unsafe { MaybeUninit::<uring_sys2::io_uring>::zeroed().assume_init() };
        ring.ring_fd = -1;
        Self { ring, features: 0u32 }
    }
    fn new(config: &mut crate::capi::xconfig_s, probe: &Probe) -> Result<Self> {
        // TODO:  SINGLE ISSUER TASKRUN, ...
        // https://manpages.debian.org/unstable/liburing-dev/io_uring_setup.2.en.html
        let mut ring = unsafe { MaybeUninit::<uring_sys2::io_uring>::zeroed().assume_init() };
        let mut params = unsafe { MaybeUninit::<uring_sys2::io_uring_params>::zeroed().assume_init() };
        params.flags = uring_sys2::IORING_SETUP_CQSIZE;
        if (config.flags & crate::capi::XCONFIG_FLAG_ATTACH_HANDLE) != 0  && probe.kernel.has_attach_wq() {
            params.wq_fd = config.attach_handle as _;
            params.flags |= uring_sys2::IORING_SETUP_ATTACH_WQ;
        } else {
            config.flags &= !crate::capi::XCONFIG_FLAG_ATTACH_HANDLE;
            params.wq_fd = -1i32 as _;
        }
        if config.kernel_poll_timeout_ms > 0 && probe.kernel.has_sq_thread_idle()  {
            params.sq_thread_idle = config.kernel_poll_timeout_ms;
            params.flags |= uring_sys2::IORING_SETUP_SQPOLL;
        } else {
            config.kernel_poll_timeout_ms = 0;
        }
        if probe.kernel.has_setup_submit_all() {
            params.flags |= uring_sys2::IORING_SETUP_SUBMIT_ALL;
        }
        let status = unsafe { uring_sys2::io_uring_queue_init_params(params.sq_entries, &mut ring as _, &mut params as _) };
        if status >= 0 {
            Ok(Self{ring, features: params.features})
        } else {
            Err(Error::from_raw_os_error(-status))
        }
    }

    fn get_sqe(&mut self) -> Option<NonNull<uring_sys2::io_uring_sqe>> {
        NonNull::new(unsafe { uring_sys2::io_uring_get_sqe(&mut self.ring as _) })
    }
}

#[repr(transparent)]
struct Sqe(*mut uring_sys2::io_uring_sqe);

impl Sqe {
    #[inline(always)]
    fn prep_read_write(&mut self, op: libc::c_int, fd: libc::c_int, addr: *mut libc::c_void, len: u32, offset: u64) {
        let sqe = unsafe { &mut *self.0 };
        sqe.opcode = op as _;
        sqe.fd = fd;
        sqe.__bindgen_anon_1.off = offset;
        sqe.__bindgen_anon_2.addr = addr as usize as _;
        sqe.len = len;
    }
    #[inline(always)]
    fn prep_read(&mut self, fd: libc::c_int, buf: *mut libc::c_void, nbytes: u32, offset: u64) {
        // TODO: use prep_read_write after tests
        unsafe { uring_sys2::io_uring_prep_read(self.0, fd, buf, nbytes, offset) };
    }
}


#[derive(Debug)]
pub struct Probe {
    is_supported: [bool; OP_FACE.len()],
    kernel: KernelVersion
}
impl Probe {
    fn has_io_uring(&self) -> bool {
        self.kernel.has_io_uring() && self.is_supported[0]
    }
}

#[derive(Debug)]
struct KernelVersion {
    major: i32,
    minor: i32,
}
impl KernelVersion {
    fn from_cstr(cstr: *const libc::c_char) -> Self {
        if cstr.is_null() {
            return Self{  major: -1, minor: 0 };
        }
        let mut offset = 0isize;
        let mut major = 0i32;
        let mut minor = 0i32;
        let mut c = unsafe { *cstr.offset(offset) as u8 as char};
        while '0' <= c && c <= '9' {
            major = (major * 10) + (c as i32 - '0' as i32);
            offset += 1;
            c = unsafe { *cstr.offset(offset) as u8 as char};
        }
        if c == '.' {
            offset += 1;
            c = unsafe { *cstr.offset(offset) as u8 as char};
            while '0' <= c && c <= '9' {
                minor = (minor * 10) + (c as i32 - '0' as i32);
                offset += 1;
                c = unsafe { *cstr.offset(offset) as u8 as char};
            }
        }
        if major > 0 && minor >= 0 {
            Self { major, minor }
        } else {
            Self{  major: -1, minor: 0 }
        }
    }
    fn new() -> Self {
        let mut uname  = unsafe { MaybeUninit::<libc::utsname>::zeroed().assume_init() };
        if (unsafe { libc::uname(&mut uname as *mut libc::utsname) } < 0) {
            log::error!("libc::uname(*mut libc::utsname) failed: {}", Error::last_os_error());
            Self { major: 0, minor: 0 }
        } else {
            Self::from_cstr(uname.release.as_ptr())
        }
    }
    fn ge(&self, major: i32, minor: i32)-> bool  {
        self.major > major || (self.major == major && self.minor >= minor)
    }
    fn has_io_uring(&self) -> bool {
        // Do not try before Linux 5.1
        self.ge(5, 1)
    }
    fn has_registered_buffers(&self) -> bool {
        self.ge(5, 19)
    }
    fn has_msg_ring(&self)  -> bool {
        self.major >= 6
    }
    fn has_recv_multishot(&self) -> bool {
        self.ge(5, 13)
    }
    fn has_multishot_accept(&self) -> bool {
        self.ge(5, 13)
    }
    fn has_attach_wq(&self) -> bool {
        self.ge(5, 6)
    }
    fn has_sq_thread_idle(&self) -> bool {
        self.ge(5, 13)
    }
    fn has_coop_taskrun(&self) -> bool {
        self.ge(5, 19)
    }
    fn has_setup_submit_all(&self) -> bool {
        self.ge(5, 18)
    }
    fn has_single_issuer(&self) -> bool {
        self.major >= 6
    }
    fn has_incremental_buffer_consumption(&self) -> bool {
        self.ge(6, 12)
    }
    fn has_clone_buffers(&self) -> bool {
        self.ge(6, 12)
    }
}

impl Probe {
    pub fn unsuported() -> Self {
        Self {
            is_supported: [false; OP_FACE.len()],
            kernel: KernelVersion { major: -1, minor: 0 }
        }
    }
    pub fn new() -> Self {
        let mut is_supported  = [false; OP_FACE.len()];
        
        let kernel = KernelVersion::new();
        if !kernel.has_io_uring() {
            return Self::unsuported();
        }
        let probe = unsafe {
            uring_sys2::io_uring_get_probe()
        };
        if probe.is_null() {
            return Self::unsuported();
        }
        for op in 0..OP_FACE.len() {
            is_supported[op] = unsafe { uring_sys2::io_uring_opcode_supported(probe, op as _) } != 0;
            // if unsafe { uring_sys2::io_uring_opcode_supported(probe, op as _) } != 0 {
            //     println!("{op}: {} : true", OP_FACE[op]);
            // }
        }
        unsafe { uring_sys2::io_uring_free_probe(probe) };
        Self{is_supported, kernel}
    }
}

const OP_FACE: &'static [&'static str] = &[
    "IORING_OP_NOP",
    "IORING_OP_READV",
    "IORING_OP_WRITEV",
    "IORING_OP_FSYNC",
    "IORING_OP_READ_FIXED",
    "IORING_OP_WRITE_FIXED",
    "IORING_OP_POLL_ADD",
    "IORING_OP_POLL_REMOVE",
    "IORING_OP_SYNC_FILE_RANGE",
    "IORING_OP_SENDMSG",
    "IORING_OP_RECVMSG",
    "IORING_OP_TIMEOUT",
    "IORING_OP_TIMEOUT_REMOVE",
    "IORING_OP_ACCEPT",
    "IORING_OP_ASYNC_CANCEL",
    "IORING_OP_LINK_TIMEOUT",
    "IORING_OP_CONNECT",
    "IORING_OP_FALLOCATE",
    "IORING_OP_OPENAT",
    "IORING_OP_CLOSE",
    "IORING_OP_FILES_UPDATE",
    "IORING_OP_STATX",
    "IORING_OP_READ",
    "IORING_OP_WRITE",
    "IORING_OP_FADVISE",
    "IORING_OP_MADVISE",
    "IORING_OP_SEND",
    "IORING_OP_RECV",
    "IORING_OP_OPENAT2",
    "IORING_OP_EPOLL_CTL",
    "IORING_OP_SPLICE",
    "IORING_OP_PROVIDE_BUFFERS",
    "IORING_OP_REMOVE_BUFFERS",
    "IORING_OP_TEE",
    "IORING_OP_SHUTDOWN",
    "IORING_OP_RENAMEAT",
    "IORING_OP_UNLINKAT",
    "IORING_OP_MKDIRAT",
    "IORING_OP_SYMLINKAT",
    "IORING_OP_LINKAT",
    "IORING_OP_MSG_RING",
    "IORING_OP_FSETXATTR",
    "IORING_OP_SETXATTR",
    "IORING_OP_FGETXATTR",
    "IORING_OP_GETXATTR",
    "IORING_OP_SOCKET",
    "IORING_OP_URING_CMD",
    "IORING_OP_SEND_ZC",
    "IORING_OP_SENDMSG_ZC",
	"IORING_OP_READ_MULTISHOT",
	"IORING_OP_WAITID",
	"IORING_OP_FUTEX_WAIT",
	"IORING_OP_FUTEX_WAKE",
	"IORING_OP_FUTEX_WAITV",
	"IORING_OP_FIXED_FD_INSTALL",
    "IORING_OP_FTRUNCATE",
    "IORING_OP_BIND",
	"IORING_OP_LISTEN",
];
