use crate::Status;

use super::{AsyncOp, AsyncOpCode, PollContext};

#[derive(Debug, Clone, Copy)]
pub struct AsyncDeadline {
    pub deadline: u64,
}
impl AsyncDeadline {
    pub(crate) fn new(deadline: u64) -> Self {
        Self { deadline }
    }
}
impl AsyncOp for AsyncDeadline {
    const OP_CODE: AsyncOpCode = AsyncOpCode::DEADLINE;
    fn poll(&mut self, cx: &PollContext) -> Status {
        if cx.now >= self.deadline {
            Status::new(0)
        } else {
            Status::pending()
        }
    }
}
