use crate::Status;

use super::{AsyncOp, PollContext};

#[derive(Debug, Clone, Copy)]
pub struct AsyncDeadline {
    pub deadline: u64,
}

impl AsyncOp for AsyncDeadline {
    fn poll(&mut self, cx: &PollContext) -> Status {
        if cx.now >= self.deadline {
            Status::new(0)
        } else {
            Status::pending()
        }
    }
}
