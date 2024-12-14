use std::io::{Error, ErrorKind, Result};
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use libc::stat;

use crate::file;
use crate::request;
use crate::{ready_fifo::ReadyFifo, Request};

#[derive(Clone)]
struct Pool {
    inner: Arc<(Mutex<Locked>, Condvar)>,
}
struct Locked {
    queue: ReadyFifo,
    workers: Vec<JoinHandle<()>>,
}
impl Locked {
    fn new(capacity: usize) -> Self {
        Self {
            queue: ReadyFifo::default(),
            workers: Vec::with_capacity(capacity),
        }
    }
}

impl Pool {
    pub(crate) fn new(max_threads: usize) -> Self {
        let mut thiz = Self {
            inner: Arc::new((Mutex::new(Locked::new(max_threads)), Condvar::new())),
        };
        {
            let mut pool = thiz.inner.0.lock().expect("Unrecoverable error");
            let min_threads = pool.workers.len();
            for id in 0..min_threads {
                pool.workers[id] = Pool::worker(thiz.clone(), id);
            }
        }
        thiz
    }

    pub(crate) fn submit(&self, req: NonNull<Request>) {
        assert!(unsafe { req.as_ref() }.is_concurrent());
        let mut pool = self.inner.0.lock().expect("Unrecoverable error");
        let was_empty = pool.queue.is_empty();
        unsafe { pool.queue.push_back(req) };
        if was_empty {
            self.inner.1.notify_one();
        }
    }
    fn next_job(&self) -> Option<NonNull<Request>> {
        let mut pool = self.inner.0.lock().expect("Unrecoverable error");
        loop {
            match unsafe { pool.queue.pop_front() } {
                Some(job) => {
                    return Some(job);
                }
                None => {
                    pool = self.inner.1.wait(pool).expect("Unrecoverable error");
                }
            }
        }
    }
    fn worker_run_job(job: &mut Request) -> i32 {
        let mut status = job.status.load(Ordering::Relaxed);
        if status == request::PENDING {
            status = match job.opcode_raw() {
                request::OP_NOOP => 0,
                request::OP_FILE_READ => file::file_io_read_sync(job),
                request::OP_FILE_WRITE => file::file_io_write_sync(job),
                _ => libc::ENOSYS,
            };
        }
        status
    }
    fn worker(pool: Pool, id: usize) -> JoinHandle<()> {
        std::thread::spawn(move || loop {
            log::trace!("Worker {id}: starting");
            match pool.next_job() {
                None => return,
                Some(mut job) => {
                    let status = Pool::worker_run_job(unsafe { job.as_mut() });
                    unsafe { Request::set_status_concurrent(job, status) };
                }
            }
            log::trace!("Worker {id}: exiting");
        })
    }
}
