use std::{
    alloc::Layout,
    future::Future,
    mem::{ManuallyDrop, MaybeUninit},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use crate::executor::{self, Executor};

pub struct Task(NonNull<Inner>);
impl Drop for Task {
    fn drop(&mut self) {
        let inner = unsafe { self.0.as_mut() } as *mut Inner;
        // std::mem::forget(self.0);
        unsafe { ((*inner).vtable.drop)(inner as _) };
        unsafe { std::alloc::dealloc(inner as _, (*inner).vtable.layout) };
    }
}
impl Task {
    fn new<F: Future + Unpin + Send>(executor: Executor, f: F) -> Option<Self> {
        InnerTyped::<F>::new(executor, f).map(Self)
    }
    fn new_local<F: Future + Unpin>(executor: Executor, f: F) -> Option<Self> {
        InnerTyped::<F>::new(executor, f).map(Self)
    }
}
impl Future for Task {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.0.as_ptr();
        unsafe { ((*inner).vtable.poll)(inner as _, cx) }
    }
}
// impl xaio_core::collection::SListNode for Inner {
//     fn offset_of_link() -> usize {
//         std::mem::offset_of!(Inner, link)
//     }
//     fn drop(ptr: Box<Self>) {
//         let _ = ptr.into();
//     }
// }

struct Inner {
    vtable: &'static VTable,
    link: xaio_core::collection::SLink,
    executor: executor::Executor,
}

struct VTable {
    layout: Layout,
    poll: fn(*mut (), &mut Context<'_>) -> Poll<()>,
    drop: fn(*mut ()),
}
struct InnerTyped<F: Future + Unpin> {
    as_inner: Inner,
    f: F,
    o: MaybeUninit<F::Output>,
}
impl<F: Future + Unpin> InnerTyped<F> {
    const VTABLE: VTable = VTable {
        layout: unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<Self>(),
                std::mem::align_of::<Self>(),
            )
        },
        poll: Self::poll,
        drop: Self::drop,
    };
    fn new(executor: Executor, f: F) -> Option<NonNull<Inner>> {
        let layout = unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<Self>(),
                std::mem::align_of::<Self>(),
            )
        };
        let thiz: *mut InnerTyped<F> = unsafe { std::alloc::alloc(layout) } as _;
        if !thiz.is_null() {
            unsafe {
                thiz.write(Self {
                    as_inner: Inner {
                        vtable: &Self::VTABLE,
                        link: xaio_core::collection::SLink::new(),
                        executor,
                    },
                    f,
                    o: MaybeUninit::uninit(),
                })
            };
            Some(unsafe { NonNull::new_unchecked(thiz as *mut Inner) })
        } else {
            None
        }
    }
    fn drop(thiz: *mut ()) {
        let thiz = thiz as *mut Self;
        unsafe { std::ptr::drop_in_place(thiz) };
    }
    fn poll(thiz: *mut (), cx: &mut Context<'_>) -> Poll<()> {
        let thiz = thiz as *mut Self;
        let f = Pin::new(unsafe { &mut (*thiz).f });
        f.poll(cx).map(|o| {
            unsafe { (*thiz).o.write(o) };
            ()
        })
    }
}
