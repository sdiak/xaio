mod context_list;

use std::ptr::NonNull;

struct Context {
    pub(self) list_prev: Option<NonNull<Context>>,
    pub(self) list_next: Option<NonNull<Context>>,
}

#[repr(align(64))]
struct Bucket {
    // Lock protecting the queue
    mutex: std::sync::Mutex<()>,
    // Linked list of threads waiting on this bucket
}
