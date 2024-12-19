use crate::sys::{unix::eventfd::EventFd, ThreadId};
use bitflags::bitflags;
use std::cell::RefCell;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Result};
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use uring_sys2;
use uring_sys2::io_uring_op as Op;

const WAKER_TOKEN: u64 = u64::MAX;
pub static PROBE: LazyLock<Probe> = LazyLock::new(Probe::new);

bitflags! {
    #[derive(Debug)]
    pub struct Feature: u32 {
        const SINGLE_MMAP = uring_sys2::IORING_FEAT_SINGLE_MMAP;
        const NODROP = uring_sys2::IORING_FEAT_NODROP;
        const SUBMIT_STABLE = uring_sys2::IORING_FEAT_SUBMIT_STABLE;
        const RW_CUR_POS = uring_sys2::IORING_FEAT_RW_CUR_POS;
        const CUR_PERSONALITY = uring_sys2::IORING_FEAT_CUR_PERSONALITY;
        const FAST_POLL = uring_sys2::IORING_FEAT_FAST_POLL;
        const POLL_32BITS = uring_sys2::IORING_FEAT_POLL_32BITS;
        const SQPOLL_NONFIXED = uring_sys2::IORING_FEAT_SQPOLL_NONFIXED;
        const EXT_ARG = uring_sys2::IORING_FEAT_EXT_ARG;
        const NATIVE_WORKERS = uring_sys2::IORING_FEAT_NATIVE_WORKERS;
        const RSRC_TAGS = uring_sys2::IORING_FEAT_RSRC_TAGS;
        const CQE_SKIP = uring_sys2::IORING_FEAT_CQE_SKIP;
        const LINKED_FILE = uring_sys2::IORING_FEAT_LINKED_FILE;
        const REG_REG_RING = uring_sys2::IORING_FEAT_REG_REG_RING;
        const RECVSEND_BUNDLE = uring_sys2::IORING_FEAT_RECVSEND_BUNDLE;
        const MIN_TIMEOUT = uring_sys2::IORING_FEAT_MIN_TIMEOUT;
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct SubmissionFlag: u8 {
        const FIXED_FILE = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_FIXED_FILE_BIT;
        const IO_DRAIN = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_IO_DRAIN_BIT;
        const IO_LINK = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_IO_LINK_BIT;
        const IO_HARDLINK = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_IO_HARDLINK_BIT;
        const ASYNC = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_ASYNC_BIT;
        const BUFFER_SELECT = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_BUFFER_SELECT_BIT;
        const CQE_SKIP_SUCCESS = 1u8 << uring_sys2::io_uring_sqe_flags_bit_IOSQE_CQE_SKIP_SUCCESS_BIT;
    }
}
bitflags! {
    #[derive(Debug)]
    pub struct SetupFlag: u32 {
        /// io_context is polled
        const IOPOLL = uring_sys2::IORING_SETUP_IOPOLL;
        /// SQ poll thread
        const SQPOLL = uring_sys2::IORING_SETUP_SQPOLL;
        /// sq_thread_cpu is valid
        const SQ_AFF = uring_sys2::IORING_SETUP_SQ_AFF;
        /// app defines CQ size
        const CQSIZE = uring_sys2::IORING_SETUP_CQSIZE;
        /// clamp SQ/CQ ring sizes
        const CLAMP = uring_sys2::IORING_SETUP_CLAMP;
        /// attach to existing wq
        const ATTACH_WQ = uring_sys2::IORING_SETUP_ATTACH_WQ;
        /// start with ring disabled
        const R_DISABLED = uring_sys2::IORING_SETUP_R_DISABLED;
        /// continue submit on error
        const SUBMIT_ALL = uring_sys2::IORING_SETUP_SUBMIT_ALL;
        /// Cooperative task running. When requests complete, they often require
        /// forcing the submitter to transition to the kernel to complete. If this
        /// flag is set, work will be done when the task transitions anyway, rather
        /// than force an inter-processor interrupt reschedule. This avoids interrupting
        /// a task running in userspace, and saves an IPI.
        const COOP_TASKRUN = uring_sys2::IORING_SETUP_COOP_TASKRUN;
        /// If COOP_TASKRUN is set, get notified if task work is available for
        /// running and a kernel transition would be needed to run it. This sets
        /// IORING_SQ_TASKRUN in the sq ring flags. Not valid with COOP_TASKRUN.
        const TASKRUN_FLAG = uring_sys2::IORING_SETUP_TASKRUN_FLAG;
        /// SQEs are 128 byte
        const SQE128 = uring_sys2::IORING_SETUP_SQE128;
        /// CQEs are 32 byte
        const CQE32 = uring_sys2::IORING_SETUP_CQE32;
        /// Only one task is allowed to submit requests
        const SINGLE_ISSUER = uring_sys2::IORING_SETUP_SINGLE_ISSUER;
        /// Defer running task work to get events.
        /// Rather than running bits of task work whenever the task transitions
        /// try to do it just before it is needed.
        const DEFER_TASKRUN = uring_sys2::IORING_SETUP_DEFER_TASKRUN;
        /// Application provides the memory for the rings
        const NO_MMAP = uring_sys2::IORING_SETUP_NO_MMAP;
        /// Register the ring fd in itself for use with
        /// IORING_REGISTER_USE_REGISTERED_RING; return a registered fd index rather
        /// than an fd.
        const REGISTERED_FD_ONLY = uring_sys2::IORING_SETUP_REGISTERED_FD_ONLY;
        /// Removes indirection through the SQ index array.
        const NO_SQARRAY = uring_sys2::IORING_SETUP_NO_SQARRAY;
    }
}

pub struct SubmissionQueue {
    ring: *mut uring_sys2::io_uring,
    tail: u32,
}

impl SubmissionQueue {
    #[inline(always)]
    fn new(ring: *mut uring_sys2::io_uring) -> Self {
        Self {
            ring,
            tail: (unsafe { *ring }).sq.sqe_tail,
        }
    }

    #[inline(always)]
    pub fn next(&mut self) -> Result<Submission> {
        let r = unsafe { &mut *self.ring };
        let next: libc::c_uint = self.tail.wrapping_add(1);
        let shift: u32 = ((r.flags & uring_sys2::IORING_SETUP_SQE128) != 0) as _;
        let khead = unsafe {
            &mut *std::mem::transmute::<*mut u32, *mut std::sync::atomic::AtomicU32>(r.sq.khead)
        };
        let head = khead.load(if (r.flags & uring_sys2::IORING_SETUP_SQPOLL) == 0 {
            Ordering::Relaxed
        } else {
            Ordering::Acquire
        });
        if next.wrapping_sub(head) <= r.sq.ring_entries {
            let sqe = unsafe {
                r.sq.sqes
                    .offset((self.tail & r.sq.ring_mask).wrapping_shl(shift) as _)
            };
            self.tail = next;
            Ok(Submission::new(sqe))
        } else {
            Err(Error::from(ErrorKind::WouldBlock))
        }
    }

    fn commit(self) -> Result<u32> {
        let r = unsafe { &mut *self.ring };
        let old_tail = r.sq.sqe_tail;
        if self.tail != old_tail {
            r.sq.sqe_tail = self.tail;
            let status = unsafe { uring_sys2::io_uring_submit(self.ring) };
            if status >= 0 {
                Ok(status as u32)
            } else {
                // Rollback tail commit
                r.sq.sqe_tail = old_tail;
                Err(Error::from_raw_os_error(-status))
            }
        } else {
            Ok(0)
        }
    }
}
pub struct CompletionQueue {
    ring: *mut uring_sys2::io_uring,
}

#[derive(Debug, Clone)]
pub struct URing {
    inner: Arc<Inner>,
}
// unsafe impl Send for URing {}
// unsafe impl Sync for URing {}

struct Inner {
    ring: uring_sys2::io_uring,
    owner_thread_id: ThreadId,
    waker: EventFd,
    shared_lock: Mutex<()>,
    local_lock: RefCell<()>,
    features: Feature,
    need_init: AtomicBool,
    waker_buffer: u64,
}

impl Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("URing")
            .field("fd", &self.ring.ring_fd)
            .field("submission_queue_depth", &self.ring.sq.ring_entries)
            .field("completion_queue_depth", &self.ring.cq.ring_entries)
            .field("flags", &SetupFlag::from_bits_retain(self.ring.flags))
            .field("features", &self.features)
            .field("need_init", &self.need_init)
            .field("owner_thread_id", &self.owner_thread_id)
            .finish()
    }
}
impl Drop for Inner {
    fn drop(&mut self) {
        if self.ring.ring_fd >= 0 {
            println!("Drop uring({})", self.ring.ring_fd);
            unsafe { uring_sys2::io_uring_queue_exit(&mut self.ring as _) };
        }
    }
}
impl URing {
    pub fn invalid() -> Self {
        let mut ring = unsafe { MaybeUninit::<uring_sys2::io_uring>::zeroed().assume_init() };
        ring.ring_fd = -1;
        Self {
            inner: Arc::new(Inner {
                ring,
                features: Feature::empty(),
                owner_thread_id: ThreadId::invalid(),
                shared_lock: Mutex::new(()),
                local_lock: RefCell::new(()),
                waker: EventFd::invalid(),
                waker_buffer: 0,
                need_init: AtomicBool::new(false),
            }),
        }
    }
    pub fn new(config: &mut Config, probe: &Probe) -> Result<Self> {
        // TODO:  SINGLE ISSUER TASKRUN, ...
        // https://manpages.debian.org/unstable/liburing-dev/io_uring_setup.2.en.html
        // https://nick-black.com/dankwiki/index.php/Io_uring
        // https://tchaloupka.github.io/during/during.io_uring.SetupFlags.html
        // https://nick-black.com/dankwiki/index.php/Io_uring
        let waker = EventFd::new(0, false)?;
        let mut ring = unsafe { MaybeUninit::<uring_sys2::io_uring>::zeroed().assume_init() };
        let mut params =
            unsafe { MaybeUninit::<uring_sys2::io_uring_params>::zeroed().assume_init() };
        params.sq_entries = config.submission_queue_depth;
        params.cq_entries = config.completion_queue_depth;
        params.flags = uring_sys2::IORING_SETUP_CQSIZE | uring_sys2::IORING_SETUP_R_DISABLED;
        if probe.kernel.ge(5, 6) {
            params.flags |= uring_sys2::IORING_SETUP_CLAMP;
        }
        if (config.flags & CONFIG_FLAG_ATTACH_HANDLE) != 0 && probe.kernel.has_attach_wq() {
            params.wq_fd = config.attach_handle as _;
            params.flags |= uring_sys2::IORING_SETUP_ATTACH_WQ;
        } else {
            config.flags &= !CONFIG_FLAG_ATTACH_HANDLE;
            params.wq_fd = -1i32 as _;
        }
        // Prefer COOP_TASKRUN to SQPOLL unless user ask for SQPOLL and it's supported
        if config.kernel_poll_timeout_ms > 0
            && probe.kernel.has_sq_thread_idle()
            && (config.flags & CONFIG_FLAG_ATTACH_SINGLE_ISSUER) != 0
        {
            params.sq_thread_idle = config.kernel_poll_timeout_ms;
            params.flags |= uring_sys2::IORING_SETUP_SQPOLL;
            todo!();
        } else {
            config.flags &= !CONFIG_FLAG_ATTACH_SINGLE_ISSUER;
            if probe.kernel.has_setup_coop_taskrun() {
                params.flags |= uring_sys2::IORING_SETUP_COOP_TASKRUN;
            }
            config.kernel_poll_timeout_ms = 0;
        }
        if probe.kernel.has_setup_submit_all() {
            params.flags |= uring_sys2::IORING_SETUP_SUBMIT_ALL;
        }

        let status = unsafe {
            uring_sys2::io_uring_queue_init_params(
                params.sq_entries,
                &mut ring as _,
                &mut params as _,
            )
        };
        if status >= 0 {
            config.submission_queue_depth = ring.sq.ring_entries;
            config.completion_queue_depth = ring.cq.ring_entries;
            let owner_thread_id = if (config.flags & CONFIG_FLAG_ATTACH_SINGLE_ISSUER) != 0 {
                ThreadId::current()
            } else {
                ThreadId::invalid()
            };
            Ok(Self {
                inner: Arc::new(Inner {
                    ring,
                    features: Feature::from_bits_retain(params.features),
                    shared_lock: Mutex::new(()),
                    local_lock: RefCell::new(()),
                    owner_thread_id,
                    waker,
                    waker_buffer: 0,
                    need_init: AtomicBool::new(true),
                }),
            })
        } else {
            Err(Error::from_raw_os_error(-status))
        }
    }

    #[inline(always)]
    fn init(&self) -> Result<()> {
        if self.inner.need_init.load(Ordering::Relaxed) {
            self.init_slow_path()
        } else {
            Ok(())
        }
    }
    #[inline(never)]
    fn init_slow_path(&self) -> Result<()> {
        let ring =
            &self.inner.ring as *const uring_sys2::io_uring as usize as *mut uring_sys2::io_uring;
        let status = unsafe { uring_sys2::io_uring_enable_rings(ring) };
        if status >= 0 {
            let fd = self.inner.waker.as_raw_fd();
            let buffer = &self.inner.waker_buffer as *const u64 as usize;
            self.submit_batch(|mut sqes| {
                sqes.next()?.prep_read(fd, buffer as _, 8, 0, WAKER_TOKEN)?;
                Ok(sqes)
            })?;
            self.inner.need_init.store(false, Ordering::Relaxed);
            Ok(())
        } else {
            Err(Error::from_raw_os_error(-status))
        }
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.inner.ring.ring_fd >= 0
    }

    #[inline(always)]
    fn ensure_locked<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(*mut uring_sys2::io_uring) -> Result<T>,
    {
        self.init()?;
        // Safety :  rules rust rules are met by
        //  - shared: Mutex::lock
        //  - thread-local: RefCell::borrow_mut
        let ring =
            &self.inner.ring as *const uring_sys2::io_uring as usize as *mut uring_sys2::io_uring;
        if self.inner.owner_thread_id.is_invalid() {
            let _ring_lock = self.inner.shared_lock.lock();
            f(ring)
        } else {
            let _ring_lock = self.inner.local_lock.borrow_mut();
            f(ring)
        }
    }
    #[inline(always)]
    pub fn submit_batch<F>(&self, f: F) -> Result<u32>
    where
        F: FnOnce(SubmissionQueue) -> Result<SubmissionQueue> + Sync,
    {
        self.ensure_locked(|ring| f(SubmissionQueue::new(ring))?.commit())
    }

    #[inline(always)]
    pub fn wake(&self) -> Result<()> {
        self.inner.waker.write(1)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Sqe(*mut uring_sys2::io_uring_sqe);

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Submission(NonNull<uring_sys2::io_uring_sqe>);

impl Submission {
    #[inline]
    fn new(sqe: *mut uring_sys2::io_uring_sqe) -> Self {
        let sqe = unsafe { &mut *sqe };
        sqe.flags = 0;
        sqe.ioprio = 0;
        sqe.__bindgen_anon_3.rw_flags = 0;
        sqe.__bindgen_anon_4.buf_index = 0;
        sqe.personality = 0;
        sqe.__bindgen_anon_5.file_index = 0;
        unsafe {
            sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut().addr3 = 0;
            sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut().__pad2[0] = 0;
            Self(NonNull::new_unchecked(sqe as *mut uring_sys2::io_uring_sqe))
        }
    }

    #[inline]
    fn prep_rw(&mut self, op: Op, fd: libc::c_int, addr: *mut libc::c_void, len: u32, offset: u64) {
        let sqe = unsafe { self.0.as_mut() };
        sqe.opcode = op as _;
        sqe.fd = fd;
        sqe.__bindgen_anon_1.off = offset;
        sqe.__bindgen_anon_2.addr = addr as usize as _;
        sqe.len = len;
    }

    #[inline(always)]
    pub fn prep_read(
        &mut self,
        fd: libc::c_int,
        buf: *mut libc::c_void,
        nbytes: u32,
        offset: u64,
        token: u64,
    ) -> Result<()> {
        self.prep_rw(Op::IORING_OP_READ, fd, buf, nbytes, offset);
        self.set_token(token)
    }

    #[inline(always)]
    pub fn token(&self) -> usize {
        unsafe { self.0.as_ref() }.user_data as _
    }
    #[inline(always)]
    pub fn set_token(&mut self, token: u64) -> Result<()> {
        if token != WAKER_TOKEN {
            unsafe { self.0.as_mut() }.user_data = token as _;
            Ok(())
        } else {
            Err(Error::from(ErrorKind::InvalidInput))
        }
    }

    #[inline(always)]
    pub fn flags(&self) -> SubmissionFlag {
        SubmissionFlag::from_bits_retain(unsafe { self.0.as_ref() }.flags)
    }
    #[inline(always)]
    pub fn set_flags(&mut self, flags: SubmissionFlag) {
        unsafe { self.0.as_mut() }.flags = flags.bits();
    }
}

impl Sqe {
    #[cfg(feature = "iouring-native-sqe")]
    #[inline]
    fn new(sqe: *mut uring_sys2::io_uring_sqe, linked: bool) -> Self {
        let sqe = unsafe { &mut *sqe };
        sqe.flags = if linked {
            uring_sys2::io_uring_sqe_flags_bit_IOSQE_IO_LINK_BIT as _
        } else {
            0
        };
        sqe.ioprio = 0;
        sqe.__bindgen_anon_3.rw_flags = 0;
        sqe.__bindgen_anon_4.buf_index = 0;
        sqe.personality = 0;
        sqe.__bindgen_anon_5.file_index = 0;
        sqe.user_data = 0; // otherwize, we could see WAKEN_TOKEN when the user does not set use_data !
        unsafe {
            sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut().addr3 = 0;
            sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut().__pad2[0] = 0;
        }
        Self(sqe as *mut uring_sys2::io_uring_sqe)
    }

    #[cfg(feature = "iouring-native-sqe")]
    #[inline]
    fn initialize(self) -> NonNull<uring_sys2::io_uring_sqe> {
        let sqe = unsafe { &mut *self.0 };
        sqe.opcode = Op::IORING_OP_NOP as _;
        sqe.flags = 0;
        sqe.ioprio = 0;
        sqe.__bindgen_anon_3.rw_flags = 0;
        sqe.__bindgen_anon_4.buf_index = 0;
        sqe.personality = 0;
        sqe.__bindgen_anon_5.file_index = 0;
        unsafe {
            sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut().addr3 = 0;
            sqe.__bindgen_anon_6.__bindgen_anon_1.as_mut().__pad2[0] = 0;
            NonNull::new_unchecked(sqe as *mut uring_sys2::io_uring_sqe)
        }
    }

    #[cfg(feature = "iouring-native-sqe")]
    #[inline]
    fn prep_rw(&mut self, op: Op, fd: libc::c_int, addr: *mut libc::c_void, len: u32, offset: u64) {
        let sqe = unsafe { &mut *self.0 };
        assert!(
            sqe.opcode == Op::IORING_OP_NOP as _,
            "Can only prep No-op submission"
        );
        sqe.opcode = op as _;
        sqe.fd = fd;
        sqe.__bindgen_anon_1.off = offset;
        sqe.__bindgen_anon_2.addr = addr as usize as _;
        sqe.len = len;
    }

    #[inline(always)]
    pub fn prep_read(
        &mut self,
        fd: libc::c_int,
        buf: *mut libc::c_void,
        nbytes: u32,
        offset: u64,
        token: usize,
    ) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "iouring-native-sqe")] {
                self.prep_rw(Op::IORING_OP_READ, fd, buf, nbytes, offset);
            } else  {
                unsafe { uring_sys2::io_uring_prep_read(self.0, fd, buf, nbytes, offset) };
            }
        }
        self.set_token(token);
    }

    #[inline(always)]
    pub fn token(&self) -> usize {
        unsafe { &*self.0 }.user_data as _
    }
    #[inline(always)]
    pub fn set_token(&self, token: usize) {
        unsafe { &mut *self.0 }.user_data = token as _;
    }
}

#[derive(Debug)]
pub struct Probe {
    is_supported: [bool; OP_FACE.len()],
    kernel: KernelVersion,
    // TODO: build supported Features
}
impl Probe {
    pub fn is_supported(&self) -> bool {
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
            return Self {
                major: -1,
                minor: 0,
            };
        }
        let mut offset = 0isize;
        let mut major = 0i32;
        let mut minor = 0i32;
        let mut c = unsafe { *cstr.offset(offset) as u8 as char };
        while '0' <= c && c <= '9' {
            major = (major * 10) + (c as i32 - '0' as i32);
            offset += 1;
            c = unsafe { *cstr.offset(offset) as u8 as char };
        }
        if c == '.' {
            offset += 1;
            c = unsafe { *cstr.offset(offset) as u8 as char };
            while '0' <= c && c <= '9' {
                minor = (minor * 10) + (c as i32 - '0' as i32);
                offset += 1;
                c = unsafe { *cstr.offset(offset) as u8 as char };
            }
        }
        if major > 0 && minor >= 0 {
            Self { major, minor }
        } else {
            Self {
                major: -1,
                minor: 0,
            }
        }
    }
    fn new() -> Self {
        let mut uname = unsafe { MaybeUninit::<libc::utsname>::zeroed().assume_init() };
        if (unsafe { libc::uname(&mut uname as *mut libc::utsname) } < 0) {
            log::error!(
                "libc::uname(*mut libc::utsname) failed: {}",
                Error::last_os_error()
            );
            Self { major: 0, minor: 0 }
        } else {
            Self::from_cstr(uname.release.as_ptr())
        }
    }
    fn ge(&self, major: i32, minor: i32) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }
    fn has_io_uring(&self) -> bool {
        // Do not try before Linux 5.5 (IORING_FEAT_SUBMIT_STABLE & IORING_FEAT_NODROP)
        self.ge(5, 5)
    }
    fn has_registered_buffers(&self) -> bool {
        self.ge(5, 19)
    }
    fn has_msg_ring(&self) -> bool {
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
    fn has_setup_coop_taskrun(&self) -> bool {
        self.ge(5, 19)
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
            kernel: KernelVersion {
                major: -1,
                minor: 0,
            },
        }
    }
    pub fn new() -> Self {
        let mut is_supported = [false; OP_FACE.len()];

        let kernel = KernelVersion::new();
        if !kernel.has_io_uring() {
            return Self::unsuported();
        }
        let probe = unsafe { uring_sys2::io_uring_get_probe() };
        if probe.is_null() {
            return Self::unsuported();
        }
        for op in 0..OP_FACE.len() {
            is_supported[op] =
                unsafe { uring_sys2::io_uring_opcode_supported(probe, op as _) } != 0;
            if unsafe { uring_sys2::io_uring_opcode_supported(probe, op as _) } != 0 {
                println!("{op}: {} : true", OP_FACE[op]);
            }
        }
        unsafe { uring_sys2::io_uring_free_probe(probe) };
        Self {
            is_supported,
            kernel,
        }
    }
}

/// Tries to reuse the sharable driver handle in `xdriver_params_s::attach_handle
pub const CONFIG_FLAG_ATTACH_SINGLE_ISSUER: u32 = 1u32 << 0;
/// Clone or share resources with the given handle
pub const CONFIG_FLAG_ATTACH_HANDLE: u32 = 1u32 << 1;
/// Close the handle on exec
pub const CONFIG_FLAG_CLOSE_ON_EXEC: u32 = 1u32 << 2;
/// Use the most efficient polling mecanism available (for example io_uring will use epoll for polling)
pub const CONFIG_FLAG_FAST_POLL: u32 = 1u32 << 3;

/// IO Driver parameters
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Config {
    /// submission queue depth
    pub submission_queue_depth: u32,
    /// completion queue depth
    pub completion_queue_depth: u32,
    /// kernel busy-polling loop timeout in milliseconds, a value of 0 deactivate kernel polling
    pub kernel_poll_timeout_ms: u32,
    /// Flags
    pub flags: u32,
    /// A sharable driver handle when (flags & XDRIVER_FLAG_ATTACH_HANDLE)
    pub attach_handle: usize,
    /// An hint on the maximal number of file descriptor
    pub max_number_of_fd_hint: u32,
    /// An hint on the maximum number of io threads (Kernel or Userspace) or 0 for defaults
    pub max_number_of_threads: u32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            submission_queue_depth: 64,
            completion_queue_depth: 256,
            kernel_poll_timeout_ms: 0,
            flags: CONFIG_FLAG_FAST_POLL | CONFIG_FLAG_CLOSE_ON_EXEC,
            attach_handle: usize::MAX,
            max_number_of_fd_hint: 1024,
            max_number_of_threads: 0,
        }
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
