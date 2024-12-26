use crate::Status;

use super::{AsyncData, PollContext};

pub struct DeadlineData {
    deadline: u64,
}

impl AsyncData for DeadlineData {
    fn poll(&mut self, cx: &PollContext) -> Status {
        if self.deadline >= cx.now {
            Status::new(0)
        } else {
            Status::pending()
        }
    }
}
