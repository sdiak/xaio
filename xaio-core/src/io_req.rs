use crate::io_buf::IoBuf;
use crate::sys::RawSd;
use std::{io::Result, mem::ManuallyDrop, ops::DerefMut, sync::atomic::Ordering};

// pub type IoReqList = crate::collection::SList<xaio_req_s>;
// pub struct IoReq(Box<xaio_req_s>);

// impl IoReq {
//     pub fn new() -> Self {
//         Self(Box::<xaio_req_s>::new(xaio_req_s::default()))
//     }

//     pub(crate) fn take(self) -> Box<xaio_req_s> {
//         self.0
//     }

//     pub fn sanity_check(&self) -> Result<()> {
//         //TODO:
//         Ok(())
//     }
// }

// const FLAG_CANCELED: u32 = 1u32 << 8;
/// Drop optimisation flags : Has a single buffer as the first field of IoReqData
const FLAG_HAS_ONE_BUFFER: u32 = 1u32 << 31;
/// Drop optimisation flags : Has at least one resource that needs droping
const FLAG_HAS_DATA: u32 = FLAG_HAS_ONE_BUFFER | 0;

#[repr(u8)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
pub enum OpCode {
    /// No operation
    NOOP, // **MUST** be first and `0`

    /// Socket recv
    RECV,
    /// Socket send
    SEND,

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

#[repr(C)]
pub struct IoReq {
    #[cfg(target_family = "windows")]
    _win_header: windows_sys::Win32::System::IO::OVERLAPPED,
    #[cfg(target_family = "unix")]
    _unix_header: usize,
    collection_slink: crate::collection::SLink,
    status: std::sync::atomic::AtomicI32,
    pub(crate) flags_and_op_code: u32,
    pub(crate) op_data: IoReqData,
}

#[repr(C)]
pub(crate) struct SocketData {
    buffer: IoBuf,
    socket: RawSd,
    done: u32,
    todo: u32,
}

#[repr(C)]
pub(crate) union IoReqData {
    pub(crate) socket: ManuallyDrop<SocketData>,
}

impl IoReq {
    pub fn new() -> Box<Self> {
        Box::new(Self::default())
    }
    pub fn sanity_check(&self) -> Result<()> {
        //TODO:
        Ok(())
    }

    #[inline(always)]
    pub fn opcode(&self) -> OpCode {
        OpCode::from(self.opcode_raw())
    }

    #[inline(always)]
    pub fn status(&self) -> i32 {
        self.status.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn opcode_raw(&self) -> u8 {
        (self.flags_and_op_code & 0xFFu32) as u8
    }

    fn __release_resources_slow_path(&mut self) {
        if (self.flags_and_op_code & FLAG_HAS_ONE_BUFFER) != 0 {
            unsafe { ManuallyDrop::drop(&mut self.op_data.socket) }
        }
    }

    pub(crate) fn _release_resources(&mut self) {
        if (self.flags_and_op_code & FLAG_HAS_DATA) != 0 {
            self.__release_resources_slow_path();
        }
        self.flags_and_op_code = 0;
    }

    #[inline(always)]
    fn __prep(&mut self, flags_and_op_code: u32) {
        self.flags_and_op_code = flags_and_op_code;
        self.collection_slink = crate::collection::SLink::new();
    }
    pub fn prep_recv(&mut self, socket: RawSd, buffer: IoBuf, len: u32) {
        if self.flags_and_op_code != 0 {
            self._release_resources();
        }
        self.__prep(FLAG_HAS_ONE_BUFFER | OP_RECV as u32);
        let socket_data = unsafe { self.op_data.socket.deref_mut() };
        socket_data.buffer = buffer;
        socket_data.socket = socket;
        socket_data.done = 0;
        socket_data.todo = len;
    }
    pub fn prep_send(&mut self, socket: RawSd, buffer: IoBuf, len: u32) {
        if self.flags_and_op_code != 0 {
            self._release_resources();
        }
        self.__prep(FLAG_HAS_ONE_BUFFER | OP_SEND as u32);
        let socket_data = unsafe { self.op_data.socket.deref_mut() };
        socket_data.buffer = buffer;
        socket_data.socket = socket;
        socket_data.done = 0;
        socket_data.todo = len;
    }
}

impl crate::collection::SListNode for IoReq {
    fn drop(ptr: Box<Self>) {
        drop(ptr)
    }
    fn offset_of_link() -> usize {
        core::mem::offset_of!(Self, collection_slink)
    }
}

impl Default for IoReq {
    fn default() -> Self {
        unsafe { std::mem::zeroed::<Self>() }
    }
}

pub(crate) const OP_NOOP: u8 = OpCode::NOOP as _;
pub(crate) const _OP_SOCKET_START_: u8 = OpCode::RECV as _;
pub(crate) const OP_RECV: u8 = OpCode::RECV as _;
pub(crate) const OP_SEND: u8 = OpCode::SEND as _;
pub(crate) const _OP_SOCKET_END_: u8 = OpCode::SEND as _;
