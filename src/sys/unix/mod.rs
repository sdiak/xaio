#[cfg_attr(
    any(target_os = "linux", target_os = "freebsd"),
    path = "event_eventfd.rs"
)]
#[cfg_attr(
    not(any(target_os = "linux", target_os = "freebsd")),
    path = "event_pipe.rs"
)]
mod event;
pub use event::*;
