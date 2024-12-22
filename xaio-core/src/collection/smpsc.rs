use std::io::Result;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::sys::ThreadId;

use super::{SLink, SList, SListNode};

const PARK_BIT: usize = 1;

pub type ParkerUnparkCb = unsafe extern "C" fn(thiz: NonNull<()>) -> ();

pub struct Receiver<T: SListNode>(Arc<Inner<T>>, crate::PhantomUnsend, crate::PhantomUnsync);

#[derive(Clone)]
pub struct Sender<T: SListNode>(Arc<Inner<T>>);
impl<T: SListNode> Sender<T> {
    pub fn send(&self, nodes: &mut SList<T>) -> bool {
        self.0.append(nodes)
    }
}

impl<T: SListNode> Receiver<T> {
    pub fn new(parker_unpark_cb: ParkerUnparkCb, parker: NonNull<()>) -> Result<Self> {
        Ok(Self(
            crate::catch_enomem(|| {
                Arc::new(Inner::<T> {
                    owner_thread_id: ThreadId::current(),
                    tail: AtomicUsize::new(0),
                    parker_unpark_cb,
                    parker,
                    _phantom: PhantomData {},
                })
            })?,
            crate::PhantomUnsend {},
            crate::PhantomUnsync {},
        ))
    }

    pub fn new_sender(&self) -> Sender<T> {
        Sender(self.0.clone())
    }

    pub fn park_begin(&self, dst: &mut SList<T>) -> usize {
        self.0.park_begin(dst)
    }

    pub fn park_end(&self, dst: &mut SList<T>) -> usize {
        self.0.park_begin(dst)
    }
}

struct Inner<T: SListNode> {
    owner_thread_id: ThreadId,
    tail: AtomicUsize,
    parker_unpark_cb: ParkerUnparkCb,
    parker: NonNull<()>,
    _phantom: PhantomData<T>,
}
impl<T: SListNode> Inner<T> {
    fn append(&self, other: &mut SList<T>) -> bool {
        let head = other.head;
        let tail = other.tail;
        if head.is_null() {
            return false;
        }
        other.head = std::ptr::null_mut();
        other.head = std::ptr::null_mut();
        let mut old_tail = self.tail.load(Ordering::Acquire);
        loop {
            unsafe { (*tail).list_update_next((old_tail & !PARK_BIT) as _, Ordering::Relaxed) };
            match self.tail.compare_exchange_weak(
                old_tail,
                head as usize,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    if old_tail == PARK_BIT {
                        unsafe { (self.parker_unpark_cb)(self.parker) };
                        return true;
                    }
                    return false;
                }
                Err(t) => {
                    old_tail = t;
                }
            }
        }
    }

    fn park_begin(&self, dst: &mut SList<T>) -> usize {
        if self.owner_thread_id != ThreadId::current() {
            // Mostly for c-binding
            eprintln!("CompletionQueue::park_begin() can only be called from the owner thread");
            std::process::abort();
        }
        let old_tail = self.tail.swap(PARK_BIT, Ordering::Acquire);
        if old_tail == 0 {
            0
        } else {
            debug_assert!(
                old_tail != PARK_BIT,
                "The park-bit can not be set at this stage"
            );
            Self::reverse_list(old_tail as _, dst)
        }
    }

    fn park_end(&self, dst: &mut SList<T>) -> usize {
        if self.owner_thread_id != ThreadId::current() {
            // Mostly for c-binding
            eprintln!("CompletionQueue::park_end() can only be called from the owner thread");
            std::process::abort();
        }
        let old_tail = self.tail.swap(0, Ordering::Acquire);
        if old_tail == PARK_BIT {
            0
        } else {
            debug_assert!(
                old_tail != 0,
                "The park-bit is either set or the queue is not empty"
            );
            Self::reverse_list(old_tail as _, dst)
        }
    }

    fn reverse_list(src_tail: *mut SLink, dst: &mut SList<T>) -> usize {
        let mut len = 0usize;
        let tail: *mut SLink = src_tail;
        let mut head = src_tail;
        let mut prev = std::ptr::null_mut::<SLink>();
        while !head.is_null() {
            len += 1;
            let next = unsafe { (*head).list_get_next(Ordering::Relaxed) };
            unsafe { (*head).list_update_next(prev, Ordering::Relaxed) };
            prev = head;
            head = next;
        }
        dst.append(&mut SList::<T> {
            head: prev,
            tail,
            _phantom: PhantomData::<T> {},
        });
        len
    }
}
