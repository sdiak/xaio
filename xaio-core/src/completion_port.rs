use crate::io_driver;
use crate::{collection::SList, IoReq};
use std::{io::Result, rc::Rc};

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        type CellType<T> = std::cell::RefCell<T>;
    } else {
        type CellType<T> = std::cell::UnsafeCell<T>;
    }
}

pub struct CompletionPort(Rc<CellType<Inner>>);

impl CompletionPort {
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            #[inline(always)]
            fn inner_mut(&self) -> std::cell::RefMut<'_, Inner> {
                self.0.borrow_mut()
            }
        } else {
            #[inline(always)]
            fn inner_mut(&self) -> &mut Inner {
                unsafe { &mut *self.0.get() }
            }
        }
    }

    pub fn submit(&self, prepared_request: Box<IoReq>) -> Result<()> {
        prepared_request.sanity_check()?;
        Ok(self.inner_mut().submit(prepared_request))
    }
    pub fn flush_submissions(&self) -> usize {
        self.inner_mut().flush_submissions()
    }
}

struct Inner {
    io_driver: io_driver::Sender,
    // submit_queue: SList<xaio_req_s>,
    // pending_submissions: usize,
}
impl Inner {
    fn submit(&mut self, prepared_request: Box<IoReq>) {
        self.io_driver.send_one(prepared_request);
    }
    fn flush_submissions(&mut self) -> usize {
        self.io_driver.flush()
    }
    // fn submit(&mut self, prepared_request: IoReq) {
    //     self.submit_queue.push_back(prepared_request.take());
    //     self.pending_submissions += 1;
    // }
    // fn flush_submissions(&mut self) -> usize {
    //     let pending_submissions = self.pending_submissions;
    //     if pending_submissions > 0 {
    //         todo!();
    //         self.pending_submissions = 0;
    //     }
    //     pending_submissions
    // }
}
