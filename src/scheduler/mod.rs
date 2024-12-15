use std::sync::Arc;

use crate::Task;
use crossbeam_deque;

pub struct TaskPool {
    inner: Arc<TaskPoolInner>,
}
struct TaskPoolInner {
    task_queue: Vec<crossbeam_deque::Worker<Task>>,
}

impl TaskPool {
    pub fn run(&self, id: usize) {
        // let prng = rand::random();
        let pool = self.inner.as_ref();
        while true {
            match pool.task_queue[id].pop() {
                Some(mut task) => {
                    if task.poll() {
                        // Done
                    }
                }
                None => {}
            }
        }
    }
}
