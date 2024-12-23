use std::io::Result;
use std::marker::PhantomData;
use std::num::NonZero;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::Unparker;
use crate::{sys::ThreadId, Unpark};

use super::{SLink, SList, SListNode};

const PARK_BIT: usize = 1;

pub struct Receiver<T: SListNode>(Arc<Inner<T>>, crate::PhantomUnsend, crate::PhantomUnsync);

#[derive(Clone)]
pub struct Sender<T: SListNode>(Arc<Inner<T>>);
impl<T: SListNode> Sender<T> {
    #[inline(always)]
    pub fn send_one(&mut self, node: Box<T>) {
        self.send_all(&mut SList::from_node(node))
    }
    #[inline(always)]
    pub fn send_all(&mut self, nodes: &mut SList<T>) {
        self.0.append(nodes);
    }
}
unsafe impl<T: SListNode> Send for Sender<T> {}

pub struct BufferedSender<T: SListNode> {
    receiver: Arc<Inner<T>>,
    buffer: SList<T>,
    buffered: usize,
    max_buffered: NonZero<usize>,
}
impl<T: SListNode> Drop for BufferedSender<T> {
    fn drop(&mut self) {
        self.flush();
    }
}
impl<T: SListNode> Clone for BufferedSender<T> {
    fn clone(&self) -> Self {
        Self::new(self.receiver.clone(), self.max_buffered)
    }
}
impl<T: SListNode> BufferedSender<T> {
    fn new(receiver: Arc<Inner<T>>, max_buffered: NonZero<usize>) -> Self {
        Self {
            receiver,
            buffer: SList::new(),
            buffered: 0,
            max_buffered,
        }
    }
    #[inline(always)]
    pub fn flush(&mut self) {
        self.receiver.append(&mut self.buffer);
        self.buffered = 0;
    }

    #[inline(always)]
    pub fn send_one(&mut self, node: Box<T>) {
        self.buffer.push_front(node);
        self.buffered += 1;
        if self.buffered >= self.max_buffered.get() {
            self.flush();
        }
    }

    pub fn send_all(&mut self, nodes: &mut SList<T>) {
        if !nodes.is_empty() {
            self.buffer.prepend(nodes);
            // Do not traverse nodes to get the length, just flush
            self.flush();
        }
    }
}
unsafe impl<T: SListNode> Send for BufferedSender<T> {}

impl<T: SListNode> Receiver<T> {
    #[cfg_attr(coverage, coverage(off))]
    pub fn try_new(target: Unparker) -> Result<Self> {
        use std::panic::AssertUnwindSafe;
        Ok(crate::catch_enomem(AssertUnwindSafe(move || {
            Self::new(target)
        }))?)
    }

    pub fn new(target: Unparker) -> Self {
        Self(
            Arc::new(Inner::<T> {
                owner_thread_id: ThreadId::current(),
                tail: AtomicUsize::new(0),
                target,
                _phantom: PhantomData {},
            }),
            crate::PhantomUnsend {},
            crate::PhantomUnsync {},
        )
    }

    pub fn new_sender(&self) -> Sender<T> {
        Sender(self.0.clone())
    }

    pub fn new_buffered_sender(&self, buffer_size_hint: usize) -> BufferedSender<T> {
        let buffer_size =
            NonZero::new(buffer_size_hint).unwrap_or(unsafe { NonZero::new_unchecked(1) });
        BufferedSender::new(self.0.clone(), buffer_size)
    }

    pub fn park_begin(&self, dst: &mut SList<T>) -> usize {
        self.0.park_begin(dst)
    }

    pub fn park_end(&self, dst: &mut SList<T>) -> usize {
        self.0.park_end(dst)
    }
}

struct Inner<T: SListNode> {
    owner_thread_id: ThreadId,
    tail: AtomicUsize,
    target: Unparker,
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
                        self.target.unpark();
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

    #[inline]
    #[cfg_attr(coverage, coverage(off))]
    fn check_current_thread(&self, method: &str) {
        if self.owner_thread_id != ThreadId::current() {
            // Mostly for c-binding
            eprintln!(
                "xaio-core::collection::smpsc::Receiver::{} can only be called from the owner thread",
                method
            );
            std::process::abort();
        }
    }
    #[inline]
    #[cfg_attr(coverage, coverage(off))]
    fn check_park_bit(&self, old_tail: usize) {
        debug_assert!(
            old_tail != PARK_BIT,
            "The park-bit can not be set at this stage"
        );
    }
    fn park_begin(&self, dst: &mut SList<T>) -> usize {
        self.check_current_thread("park_begin()");
        // println!("old_tail={}", self.tail.load(Ordering::Relaxed));
        let old_tail = self.tail.swap(PARK_BIT, Ordering::Acquire);
        if old_tail == 0 {
            0
        } else {
            // println!(" old_tail={old_tail}");
            self.check_park_bit(old_tail);
            Self::reverse_list(old_tail as _, dst)
        }
    }

    fn park_end(&self, dst: &mut SList<T>) -> usize {
        self.check_current_thread("park_end()");
        let old_tail = self.tail.swap(0, Ordering::Acquire);
        if old_tail <= PARK_BIT {
            0
        } else {
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

#[cfg(test)]
mod test {
    use std::thread::JoinHandle;

    use super::*;

    struct IntNode {
        pub val: i32,
        link: SLink,
    }
    impl IntNode {
        fn new(val: i32) -> Box<Self> {
            Box::new(Self {
                val,
                link: SLink::new(),
            })
        }
    }
    impl SListNode for IntNode {
        fn offset_of_link() -> usize {
            core::mem::offset_of!(IntNode, link)
        }
        fn drop(ptr: Box<Self>) {
            drop(ptr);
        }
    }

    #[derive(Clone)]
    struct Thread(std::thread::Thread);
    impl Unpark for Thread {
        fn unpark(&self) {
            self.0.unpark();
            // println!("Unpark");
        }
    }
    #[test]
    fn test_single_thread() {
        let me = Thread(std::thread::current());
        let target = Unparker::new(me);
        let reveiver: Receiver<IntNode> = Receiver::try_new(target).unwrap();

        const N_MSG: usize = 1000;
        let mut send1 = reveiver.new_sender();
        let t1 = std::thread::spawn(move || {
            send1.send_all(&mut SList::<IntNode>::new());
            for i in 0..N_MSG {
                send1.send_one(IntNode::new(i as i32));
            }
        });

        let mut messages = SList::<IntNode>::new();
        let mut n_msg = 0;
        while n_msg < N_MSG {
            let n = reveiver.park_begin(&mut messages);
            if n > 0 {
                n_msg += n;
            } else {
                std::thread::park();
            }
            n_msg += reveiver.park_end(&mut messages);
        }

        for i in 0..N_MSG as i32 {
            assert_eq!(messages.pop_front().unwrap().val, i);
        }
        t1.join();
    }
    #[test]
    fn test_single_thread_batch() {
        let me = Thread(std::thread::current());
        let target = Unparker::new(me);
        let reveiver: Receiver<IntNode> = Receiver::try_new(target).unwrap();

        const N_MSG: usize = 1000;
        let mut send1 = reveiver.new_buffered_sender(3);
        let t1 = std::thread::spawn(move || {
            send1.send_all(&mut SList::<IntNode>::new());
            send1.send_all(&mut SList::<IntNode>::from_node(IntNode::new(0i32)));
            for i in 1..N_MSG {
                send1.send_one(IntNode::new(i as i32));
                if i == 2 {
                    send1 = send1.clone();
                }
            }
        });

        let mut messages = SList::<IntNode>::new();
        let mut n_msg = 0;
        while n_msg < N_MSG {
            let n = reveiver.park_begin(&mut messages);
            if n > 0 {
                n_msg += n;
            } else {
                std::thread::park();
            }
            n_msg += reveiver.park_end(&mut messages);
        }

        for i in 0..N_MSG as i32 {
            assert_eq!(messages.pop_front().unwrap().val, i);
        }
        t1.join().unwrap();
    }
    #[test]
    fn test_multi_thread() {
        let me = Thread(std::thread::current());
        let target = Unparker::new(me);
        let reveiver: Receiver<IntNode> = Receiver::try_new(target).unwrap();

        const N_THREAD: usize = 3;
        const N_MSG_PER_THREAD: usize = 1000;
        let mut threads = Vec::<JoinHandle<()>>::new();

        for _ in 0..N_THREAD {
            let mut send = reveiver.new_sender();
            threads.push(std::thread::spawn(move || {
                for i in 0..N_MSG_PER_THREAD {
                    send.send_one(IntNode::new(i as i32));
                }
            }));
        }

        const N_MSG: usize = N_MSG_PER_THREAD * N_THREAD;
        let mut messages = SList::<IntNode>::new();
        let mut n_park = 0;
        let mut n_msg = 0;
        while n_msg < N_MSG {
            let n = reveiver.park_begin(&mut messages);
            if n > 0 {
                n_msg += n;
            } else {
                std::thread::park();
                n_park += 1;
            }
            n_msg += reveiver.park_end(&mut messages);
        }

        for t in threads {
            t.join().unwrap();
        }
        println!("n_park={n_park}");
    }
}
