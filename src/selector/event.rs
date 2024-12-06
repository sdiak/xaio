use crate::selector::Interest;

#[repr(C, packed(1))]
pub struct Event {
    events: Interest,
    token: usize,
}