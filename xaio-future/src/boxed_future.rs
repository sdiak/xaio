use std::{
    alloc::Layout,
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

pub struct BoxedFuture(NonNull<Inner>);
impl Drop for BoxedFuture {
    fn drop(&mut self) {
        let inner = self.0.as_ptr();
        unsafe { ((*inner).vtable.drop)(inner as _) };
        unsafe { std::alloc::dealloc(inner as _, (*inner).vtable.layout) };
    }
}
impl BoxedFuture {
    fn new<F: Future + Unpin + Send>(f: F) -> Option<Self> {
        InnerTyped::<F>::new(f).map(Self)
    }
    fn new_local<F: Future + Unpin>(f: F) -> Option<Self> {
        InnerTyped::<F>::new(f).map(Self)
    }
}
impl Future for BoxedFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.0.as_ptr();
        unsafe { ((*inner).vtable.poll)(inner as _, cx) }
    }
}
struct Inner {
    vtable: &'static VTable,
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
    fn new(f: F) -> Option<NonNull<Inner>> {
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
// impl Future for BoxedFuture {
//     type Output = ();
//     fn poll(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Self::Output> {
//         let inner = unsafe { self.0.as_mut() };
//         self.inner.
//     }
// }
