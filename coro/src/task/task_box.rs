use crate::{future::State, ptr::Ptr};
use std::{
    alloc::Layout, cell::UnsafeCell, mem::MaybeUninit, panic::AssertUnwindSafe, thread::Thread,
};

use super::{Context, Task};

thread_local! { static CURRENT: Option<TaskBox> = const { None }; }

#[repr(transparent)]
#[derive(Debug)]
pub struct TaskBox(Ptr<UnsafeCell<TaskInnerErazed>>);

pub(crate) unsafe fn __spawn<T: Task>(task: T) -> Option<(TaskBox, Ptr<UnsafeCell<TaskInner<T>>>)> {
    TaskInner::try_new(task).map(|(a, b)| (TaskBox(a), b))
}

impl TaskBox {
    // pub(crate) fn resume(&self, cx: Context) -> bool {
    //     self.0
    // }
    fn __clone(&self) -> TaskBox {
        Self(unsafe { self.0.clone_raw() })
    }
    pub(crate) fn resume(&mut self) -> bool {
        let thiz = self.0.as_mut().get_mut();
        println!("Resume");
        let x = (thiz.vtable.resume)(thiz as *mut TaskInnerErazed as _, &mut thiz.context);
        println!("  Resule => {x}");
        x
    }
}

#[derive(Debug, Clone, Copy)]
struct VTable {
    layout: std::alloc::Layout,
    resume: fn(*mut (), &mut Context) -> bool,
    drop: fn(*mut ()),
}
// impl UnwindSafe for VTable {}

#[derive(Debug)]
struct TaskInnerErazed {
    vtable: &'static VTable,
    // executor: Mutex<&'static Executor>,
    parent: std::result::Result<TaskBox, Thread>,
    context: Context,
}

pub(crate) struct TaskInner<T: Task> {
    as_inner: TaskInnerErazed,
    task: T,
    output: MaybeUninit<T::Output>,
}

impl<T: Task> TaskInner<T> {
    const VTABLE: VTable = VTable {
        layout: unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<UnsafeCell<Self>>(),
                std::mem::align_of::<UnsafeCell<Self>>(),
            )
        },
        resume: Self::resume,
        drop: Self::drop,
    };
    pub(crate) fn state(&self, order: std::sync::atomic::Ordering) -> State {
        self.as_inner.context.state(order)
    }
    pub(crate) fn wait(self) -> T::Output {
        loop {
            match self.state(std::sync::atomic::Ordering::Acquire) {
                State::Pending => {
                    std::thread::park();
                }
                State::Cancelling => {
                    std::thread::park();
                }
                State::Ready => return unsafe { self.output.assume_init() },
                State::Paniced => panic!("Producer paniced"),
            }
        }
    }
    fn try_new(
        task: T,
    ) -> Option<(
        Ptr<UnsafeCell<TaskInnerErazed>>,
        Ptr<UnsafeCell<TaskInner<T>>>,
    )> {
        let thiz: *mut UnsafeCell<TaskInner<T>> =
            unsafe { std::alloc::alloc(Self::VTABLE.layout) } as _;
        if !thiz.is_null() {
            CURRENT.with(|parent| {
                let parent = if parent.is_some() {
                    Ok(TaskBox::__clone(parent.as_ref().unwrap()))
                } else {
                    Err(std::thread::current())
                };
                unsafe {
                    thiz.write(UnsafeCell::new(Self {
                        as_inner: TaskInnerErazed {
                            vtable: &Self::VTABLE,
                            context: Context::new(),
                            parent, // Safety child always ends after parent. TODO: handle orphan
                        },
                        task,
                        output: MaybeUninit::uninit(),
                    }))
                };
            });
            Some(unsafe {
                (
                    Ptr::from_raw_unchecked(thiz as *mut UnsafeCell<TaskInnerErazed>),
                    Ptr::from_raw_unchecked(thiz),
                )
            })
        } else {
            None
        }
    }
    fn drop(thiz: *mut ()) {
        let thiz = thiz as *mut Self;
        unsafe { std::ptr::drop_in_place(thiz) };
    }
    fn __wake_parent(&self) -> bool {
        match &self.as_inner.parent {
            Ok(task) => todo!(),
            Err(thread) => thread.unpark(),
        }
        true
    }
    fn resume(thiz: *mut (), cx: &mut Context) -> bool {
        let thiz = unsafe { &mut *(thiz as *mut Self) };

        match std::panic::catch_unwind(AssertUnwindSafe(|| thiz.task.resume(cx))) {
            Ok(poll) => {
                if let Some(o) = poll {
                    // TODO: catch_unwind
                    thiz.output.write(o);
                    thiz.as_inner
                        .context
                        .state
                        .store(State::Ready as u8, std::sync::atomic::Ordering::Release);
                    thiz.__wake_parent()
                } else {
                    false
                }
            }
            Err(_) => {
                thiz.as_inner
                    .context
                    .state
                    .store(State::Paniced as u8, std::sync::atomic::Ordering::Release);
                thiz.__wake_parent()
            }
        }
    }
}
