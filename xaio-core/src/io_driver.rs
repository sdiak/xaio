use crate::{collection::SList, IoReq, Uniq};

#[derive(Debug, Clone, Copy)]
pub struct IoDriverConfig {
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
impl IoDriverConfig {
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
pub trait IoDriver {
    type Sender: IoReqSender;

    fn new_sender(&self) -> Self::Sender;
}

pub struct TmpIoDriverSender {}
impl TmpIoDriverSender {
    pub fn submit(&mut self, _batch: &mut SList<IoReq>) {
        todo!()
    }
}
pub trait IoReqSender: Clone {
    // fn can_serve(&self, opcode: OpCode) -> bool;

    /// Sends one request to the driver (the request might be buffered until the call to `IoDriverSender::flush`)
    fn send_one(&self, request: Uniq<IoReq>);

    /// Sends a batch of requests to the driver (the requests might be buffered until the call to `IoDriverSender::flush`)
    fn send_many(&self, requests: &mut crate::collection::SList<IoReq>) {
        while let Some(request) = requests.pop_front() {
            self.send_one(request);
        }
    }

    /// Flushes any batched requests and returns an estimation of the
    /// amount of requests buffered before flushing
    fn flush(&self) -> usize;
}
