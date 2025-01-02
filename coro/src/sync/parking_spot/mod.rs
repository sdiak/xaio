mod context;
pub(crate) mod tagged_ptr;
use std::sync::{atomic::AtomicUsize, Mutex};

pub use context::*;

pub(self) mod wait_list;

pub struct ParkingSpot {
    key: AtomicUsize,
    mutex: Mutex<()>,
}
