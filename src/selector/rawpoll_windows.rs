use winapi::um::winsock2::{
    accept, bind, closesocket, connect, getsockname, getsockopt, htonl, ioctlsocket, listen, recv,
    send, WSAGetLastError, WSAPoll, WSASocketW, WSAStartup, INVALID_SOCKET, SOCK_STREAM,
    SOL_SOCKET, SO_ERROR, WSADATA, WSAENOTSOCK, WSA_FLAG_NO_HANDLE_INHERIT,
};
pub use winapi::um::winsock2::{POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI, SOCKET as SocketFD, WSAPOLLFD as PollFD};

pub fn socket_is_valid(s: SocketFD) -> bool {
    s != INVALID_SOCKET
}

pub fn sys_poll(pfd: &mut [PollFD], timeout: i32) -> Result<usize, std::io::Error> {
    let poll_result = unsafe { WSAPoll(pfd.as_mut_ptr(), pfd.len() as _, timeout as _) };
    if poll_result < 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(poll_result as usize)
    }
}
