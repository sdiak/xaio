use std::{cell::UnsafeCell, rc::Rc};

use crate::{
    future::Future,
    task::{FnTask, Task, TaskBox},
};

// thread_local! { static CURRENT: Option<Rc<UnsafeCell<Inner>>> = const { None }; }

struct Executor {
    local_tasks: Vec<TaskBox>,
}

struct Inner {}

pub(crate) fn tmp_spawn_executor_4task<T: Task>(initial_task: T) -> Future<T> {
    let (initial_task, future) = unsafe { crate::task::__spawn(initial_task).unwrap() };
    let mut executor = Executor {
        local_tasks: vec![initial_task],
    };
    let _ = std::thread::spawn(move || {
        while let Some(mut task) = executor.local_tasks.pop() {
            if !task.resume() {
                executor.local_tasks.push(task);
            }
        }
    });
    future
}

pub(crate) fn tmp_spawn_executor<T, O>(task: T) -> Future<FnTask<T, O>>
where
    T: FnOnce() -> O,
{
    let task = FnTask::new(task);
    tmp_spawn_executor_4task(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let future = tmp_spawn_executor(|| {
            println!("Task: start");
            std::thread::sleep(std::time::Duration::from_millis(1000));
            println!("Task: end");
            42
        });
        println!("Will-wait: ");
        println!("Wait: {}", future.wait());

        // std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
