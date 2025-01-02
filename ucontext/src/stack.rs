struct ValgrindStackId {
    #[cfg(test)]
    id: usize,
}

impl ValgrindStackId {
    fn register(_size: usize, _base: *mut u8) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                Self {
                    id: 0, // TODO:
                }
            } else {

                Self {}
            }
        }
    }
}
pub struct Stack {
    size: usize,
    base: *mut u8,
    valgrind_stack_id: ValgrindStackId,
}
impl Drop for Stack {
    fn drop(&mut self) {
        self.deallocate();
    }
}

impl Stack {
    pub fn new() -> Option<Stack> {
        let size: usize = 1048576;
        let base =
            unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(size, 4096)) };
        if base.is_null() {
            None
        } else {
            Some(Self {
                size,
                base,
                valgrind_stack_id: ValgrindStackId::register(size, base),
            })
        }
    }
    fn deallocate(&mut self) {
        todo!()
    }
}
