use crate::collection::smpsc;
use crate::IoReq;

pub type Sender = smpsc::BufferedSender<IoReq>;

pub struct IoDriver {}
