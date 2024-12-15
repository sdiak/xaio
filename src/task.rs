use crate::status;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::AtomicIsize;

struct Worker {}
