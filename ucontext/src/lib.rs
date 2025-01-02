pub mod stack;

mod sys;

pub struct UContext {
    pub(crate) stack_pointer: *mut usize,
}
