#![allow(dead_code)]

#[cfg(target_family = "unix")]
mod main_unix;
#[cfg(target_family = "unix")]
pub use main_unix::*;

#[cfg(target_family = "windows")]
mod main_windows;
#[cfg(target_family = "windows")]
pub use main_windows::*;
