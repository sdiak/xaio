use crate::selector::Interest;

#[repr(C, packed(1))]
#[derive(Debug)]
pub struct Event {
    pub events: Interest,
    pub token: u64,
}

// struct epoll_event {
//     uint32_t      events;  /* Epoll events */
//     epoll_data_t  data;    /* User data variable */
// };
