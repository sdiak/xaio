pub trait FutureListener: Send {
    fn notify(&self, token: usize);
}

struct FutureListenerVTable {
    layout: std::alloc::Layout,
    notify: unsafe fn(*const (), usize),
    drop: unsafe fn(*mut ()),
}
pub struct FutureListenerErased {
    vtable: &'static FutureListenerVTable,
}
impl FutureListenerErased {
    pub(crate) fn notify(&self, token: usize) {
        unsafe { (self.vtable.notify)(self as *const Self as _, token) };
    }
}
pub struct FutureListener2<FL: FutureListener> {
    erased: FutureListenerErased,
    listener: FL,
}
impl<FL: FutureListener> FutureListener2<FL> {
    const VTABLE: FutureListenerVTable = FutureListenerVTable {
        layout: unsafe {
            std::alloc::Layout::from_size_align_unchecked(
                std::mem::size_of::<FL>(),
                std::mem::align_of::<FL>(),
            )
        },
        notify: Self::notify,
        drop: Self::drop,
    };
    pub fn new(listener: FL) -> Self {
        Self {
            erased: FutureListenerErased {
                vtable: &Self::VTABLE,
            },
            listener,
        }
    }
    unsafe fn notify(thiz: *const (), token: usize) {
        let thiz = unsafe { &*(thiz as *const Self) };
        thiz.listener.notify(token);
    }
    unsafe fn drop(thiz: *mut ()) {
        unsafe { std::ptr::drop_in_place(thiz as *mut Self) };
    }
}
