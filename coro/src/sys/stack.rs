pub struct Stack {
    pub(crate) size: isize,
    pub(crate) base: *mut u8,
}

impl Stack {
    pub fn new() -> Option<Stack> {
        let base = unsafe {
            std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(1048576, 4096))
        };
        if base.is_null() {
            None
        } else {
            Some(Self {
                size: 1048576,
                base,
            })
        }
    }
}
