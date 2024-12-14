use std::{os::fd::RawFd, ptr::NonNull, sync::atomic::Ordering};

use crate::{selector::Interest, RawSocketFd};

pub(super) const PENDING: i32 = i32::MIN;
pub(super) const UNKNOWN: i32 = i32::MIN + 1;

#[repr(u8)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
pub enum OpCode {
    /// No operation
    NOOP, // **MUST** be first and `0`
    /// Socket poll
    SOCKET_POLL,
    /// Socket recv
    SOCKET_RECV,
    /// Socket send
    SOCKET_SEND,

    FILE_READ,
    FILE_WRITE,
    /// An invalid op-code
    INVALID, // **MUST** be last
}
impl From<u8> for OpCode {
    #[inline(always)]
    fn from(value: u8) -> Self {
        if value <= OpCode::INVALID as u8 {
            unsafe { std::mem::transmute::<u8, OpCode>(value) }
        } else {
            OpCode::INVALID
        }
    }
}
impl From<OpCode> for u8 {
    #[inline(always)]
    fn from(value: OpCode) -> Self {
        value as _
    }
}

pub(crate) const OP_NOOP: u8 = OpCode::NOOP as _;
const _OP_SOCKET_START: u8 = OpCode::SOCKET_POLL as _;
pub(crate) const OP_SOCKET_POLL: u8 = OpCode::SOCKET_POLL as _;
pub(crate) const OP_SOCKET_RECV: u8 = OpCode::SOCKET_RECV as _;
pub(crate) const OP_SOCKET_SEND: u8 = OpCode::SOCKET_SEND as _;
const _OP_SOCKET_END: u8 = OpCode::SOCKET_SEND as _;
pub(crate) const OP_FILE_READ: u8 = OpCode::FILE_READ as _;
pub(crate) const OP_FILE_WRITE: u8 = OpCode::FILE_WRITE as _;

#[repr(C)]
// #[derive(Clone)]
pub struct Request {
    #[cfg(target_os = "windows")]
    win_header: windows_sys::Win32::System::IO::OVERLAPPED,
    // prv__cp: *mut xcp_s,
    // pub(crate) owner: Option<RefCell<RingInner>>,
    // request status
    pub(crate) status: i32,
    // reques status set by a concurrent thread
    pub(crate) concurrent_status: std::sync::atomic::AtomicI32,
    flags_and_op_code: u32,
    list_next: std::sync::atomic::AtomicUsize,
    pub(crate) op: RequestData,
}
impl Default for Request {
    fn default() -> Self {
        unsafe { std::mem::MaybeUninit::zeroed().assume_init() }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SocketRequest {
    /// The socket
    pub(crate) socket: RawSocketFd,
    /// The interests
    pub(crate) interests: u16,
    /// The events
    pub(crate) events: u16,
    /// Amount of read or write to do
    pub(crate) todo: u32,
    /// Amount of read or write already done
    pub(crate) done: u32,
    /// The read or write buffer
    pub(crate) buffer: *mut u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileIORequest {
    /// The fd
    pub(crate) fd: RawFd,
    /// Amount of read or write to do
    pub(crate) todo: u32,
    /// File position
    pub(crate) offset: u64,
    /// The read or write buffer
    pub(crate) buffer: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union RequestData {
    pub(crate) socket: SocketRequest,
    pub(crate) file_io: FileIORequest,
}

#[repr(C)]
pub struct RequestHandle {
    token: usize,
}

impl Request {
    const IN_A_LIST_BIT: usize = 1usize;

    #[inline]
    pub fn in_a_list(&self) -> bool {
        (self.list_next.load(Ordering::Relaxed) & Request::IN_A_LIST_BIT) != 0
    }
    #[inline]
    pub fn list_set_next(&mut self, next: *mut Request, order: Ordering) {
        debug_assert!(!self.in_a_list());
        self.list_next
            .store((next as usize) | Request::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub fn list_update_next(&mut self, next: *mut Request, order: Ordering) {
        debug_assert!(self.in_a_list());
        self.list_next
            .store((next as usize) | Request::IN_A_LIST_BIT, order);
    }
    #[inline]
    pub fn list_get_next(&self, order: Ordering) -> *mut Request {
        (self.list_next.load(order) & !Request::IN_A_LIST_BIT) as *mut Request
    }
    #[inline]
    pub fn list_pop_next(&self, order: Ordering) -> *mut Request {
        let old_next = (self.list_next.load(order) & !Request::IN_A_LIST_BIT) as *mut Request;
        self.list_next.store(0usize, Ordering::Relaxed);
        old_next
    }

    #[inline(always)]
    pub fn opcode(&self) -> OpCode {
        OpCode::from(self.opcode_raw())
    }

    #[inline(always)]
    pub fn opcode_raw(&self) -> u8 {
        (self.flags_and_op_code & 0xFFu32) as u8
    }

    #[inline(always)]
    #[allow(clippy::manual_range_contains)]
    pub fn is_a_socket_op(&self) -> bool {
        let opcode: u8 = self.opcode_raw();
        _OP_SOCKET_START <= opcode && opcode <= _OP_SOCKET_END
    }

    #[cfg(target_family = "unix")]
    pub(crate) fn get_socket(&self) -> std::mem::ManuallyDrop<socket2::Socket> {
        use std::os::fd::FromRawFd;
        std::mem::ManuallyDrop::new(unsafe {
            socket2::Socket::from_raw_fd(self.op.socket.socket.inner as _)
        })
    }
    #[cfg(target_family = "windows")]
    pub(crate) fn get_socket(&self) -> std::mem::ManuallyDrop<socket2::Socket> {
        use std::os::windows::io::FromRawSocket;
        std::mem::ManuallyDrop::new(unsafe {
            socket2::Socket::from_raw_fd(self.op.socket.socket.inner as _)
        })
    }

    // }
    /*
    pub fn set_status(self, status: i32) -> bool {
        if status == PENDING {
            panic!("Invalid status");
        }
        let r = self
            .status
            .compare_exchange(PENDING, status, Ordering::Release, Ordering::Relaxed);
        r.is_ok()
    }
    pub fn set_status_local(self, status: i32) -> bool {
        if status == PENDING {
            panic!("Invalid status");
        }
        if self.status.load(Ordering::Relaxed) == PENDING {
            self.status.store(status, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
    /// Cancel the Sub and consume it
    pub fn cancel(self) -> bool {
        let r = self.status.compare_exchange(
            PENDING,
            libc::ECANCELED,
            Ordering::Release,
            Ordering::Relaxed,
        );
        r.is_ok()
    }
    */
}
