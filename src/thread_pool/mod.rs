use std::mem::{ManuallyDrop, MaybeUninit};
use std::num::NonZero;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use crate::file;
use crate::request;
use crate::{ready_fifo::ReadyFifo, Request};

static IO_POOL: LazyLock<Pool> = LazyLock::new(Pool::default);
static WORKER_STACK_SIZE: usize = 262144usize;

pub(crate) fn submit_io_work(
    completion_queue: &crate::request_queue::RequestQueue,
    mut work: NonNull<Request>,
) -> crate::Status {
    unsafe { work.as_mut().set_concurrent(completion_queue) };
    IO_POOL.submit(work);
    crate::Status::new(0) // TODO: forward errors
}

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
    fn new(max_threads: usize) -> Self {
        let thiz = Self {
            inner: Arc::new((Mutex::new(Locked::new(max_threads)), Condvar::new())),
        };
        {
            let mut pool = thiz.inner.0.lock().expect("Unrecoverable error");
            let min_threads = pool.workers.len();
            for id in 0..min_threads {
                pool.workers[id] = Pool::worker(thiz.clone(), id).expect("TODO: error forwarding");
            }
        }
        thiz
    }

    fn submit(&self, req: NonNull<Request>) {
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

    fn run_rust_work(work: &mut crate::request::RustWork) -> i32 {
        match work.work.take() {
            Some(work) => {
                work();
                0 // TODO:
            }
            None => 0,
        }
    }
    fn worker_run_job(job: &mut Request) -> i32 {
        let mut status = job.status.load(Ordering::Relaxed);
        if status == request::PENDING {
            status = match job.opcode_raw() {
                request::OP_NOOP => 0,
                request::OP_IO_WORK => Pool::run_rust_work(unsafe { &mut job.op.rust_work }),
                request::OP_FILE_READ => file::file_io_read_sync(job),
                request::OP_FILE_WRITE => file::file_io_write_sync(job),
                _ => libc::ENOSYS,
            };
        }
        status
    }
    fn worker(pool: Pool, id: usize) -> std::io::Result<JoinHandle<()>> {
        std::thread::Builder::new()
            .stack_size(WORKER_STACK_SIZE)
            .spawn(move || loop {
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

impl Default for Pool {
    fn default() -> Self {
        // use std::env;
        // TODO: let env_value = match  {
        //     env::var("XAIO_IO_POOL_SIZE")
        // };
        let mut hw_parallelism = std::thread::available_parallelism()
            .unwrap_or(unsafe { NonZero::new_unchecked(1usize) })
            .get();
        hw_parallelism = hw_parallelism * 2 + 1;
        if hw_parallelism > 1024 {
            hw_parallelism = 1024;
        }
        Self::new(hw_parallelism)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple() {
        let cq = crate::request_queue::RequestQueue::new().expect("Unrecoverable");
        submit_io_work(&cq, work);
    }
}
