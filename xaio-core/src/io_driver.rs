use crate::{collection::SList, IoReq};

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
    fn send_one(&self, request: Box<IoReq>);

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
