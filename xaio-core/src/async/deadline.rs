use crate::Status;

use super::{AsyncData, PollContext};

pub struct AsyncDeadline {
    pub deadline: u64,
}

impl AsyncData for AsyncDeadline {
    fn poll(&mut self, cx: &PollContext) -> Status {
        if self.deadline >= cx.now {
            Status::new(0)
        } else {
            Status::pending()
        }
    }
}
