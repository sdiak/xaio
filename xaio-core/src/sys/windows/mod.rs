pub type RawSocket = std::os::windows::raw::SOCKET;
pub type RawFd = std::os::windows::raw::HANDLE;

pub const INVALID_RAW_SOCKET: RawSocket = -1 as _;
pub const INVALID_RAW_FD: RawFd = -1 as usize as _;
