use std::{
    cell::{Cell, UnsafeCell},
    mem::ManuallyDrop,
    ptr::NonNull,
    thread::LocalKey,
};

use super::tagged_ptr::TaggedPointer;

#[repr(usize)]
pub enum ContextKind {
    Thread = 0,
    Coroutine = 1,
}

thread_local! {
    static THREAD: UnsafeCell<Context> = UnsafeCell::new(Context::__current_thread());
    static COROUTINE: Cell<*mut Context> = const { Cell::new(std::ptr::null_mut()) };
}

type Coroutine = usize; // TODO:
union ContextData {
    thread: ManuallyDrop<std::thread::Thread>,
    coroutine: ManuallyDrop<Coroutine>,
}
pub struct Context {
    list_next: TaggedPointer<Context>,
    data: ContextData,
}
impl Context {
    pub(crate) fn with_current<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        let coro = COROUTINE.get();
        if coro.is_null() {
            // SAFETY: only a single borrow using Context::with_current
            THREAD.with(|cell| f(unsafe { &mut *cell.get() }))
        } else {
            // SAFETY: only a single borrow using Context::with_current
            f(unsafe { &mut *coro })
        }
    }
    fn __current_thread() -> Self {
        Self {
            list_next: TaggedPointer::null(ContextKind::Thread as usize),
            data: ContextData {
                thread: ManuallyDrop::new(std::thread::current()),
            },
        }
    }

    #[inline(always)]
    pub fn kind(&self) -> ContextKind {
        unsafe { std::mem::transmute::<usize, ContextKind>(self.list_next.tag()) }
    }
    #[inline]
    pub fn unpark(&self) {
        match self.kind() {
            ContextKind::Thread => {
                unsafe { self.data.thread.unpark() };
            }
            ContextKind::Coroutine => {
                todo!()
            }
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        match self.kind() {
            ContextKind::Thread => {
                unsafe { ManuallyDrop::<std::thread::Thread>::drop(&mut self.data.thread) };
            }
            ContextKind::Coroutine => {
                unsafe { ManuallyDrop::<Coroutine>::drop(&mut self.data.coroutine) };
            }
        }
    }
}
