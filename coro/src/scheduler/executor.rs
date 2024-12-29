use std::{cell::UnsafeCell, rc::Rc};

use crate::task::{FnTask, Task, TaskBox};

// thread_local! { static CURRENT: Option<Rc<UnsafeCell<Inner>>> = const { None }; }

struct Executor {
    local_tasks: Vec<TaskBox>,
}

struct Inner {}

pub(crate) fn tmp_spawn_executor_4task<T: Task>(initial_task: T) {
    let (initial_task, _) = unsafe { crate::task::__spawn(initial_task).unwrap() };
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
}

pub(crate) fn tmp_spawn_executor<F, O>(task: F)
where
    F: FnOnce() -> O,
{
    let task = FnTask::new(task);
    tmp_spawn_executor_4task(task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        tmp_spawn_executor(|| println!("It works !"));

        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
