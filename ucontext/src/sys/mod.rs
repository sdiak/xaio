extern crate page_size;

cfg_if::cfg_if! {
    if #[cfg(target_family = "unix")] {
        mod unix;
        pub use unix::*;
    }
}
struct ValgrindStackId {
    #[cfg(test)]
    id: usize,
}

use std::ptr::NonNull;

#[cfg(test)]
use crabgrind as cg;

impl ValgrindStackId {
    const INVALID: usize = usize::MAX;

    fn register(&mut self, _size: usize, _base: *mut u8) {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                debug_assert!(self.id == Self::INVALID);
                self.id = if cg::run_mode() != cg::RunMode::Native {
                    cg::memcheck::stack::register(_base as _, unsafe { _base.offset(_size as _) } as _)
                } else {
                    usize::MAX
                }
            }
        }
    }
    fn deregister(&mut self) {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                if self.id != Self::INVALID {
                    cg::memcheck::stack::deregister(self.id);
                    self.id = Self::INVALID;
                }
            }
        }
    }
    #[inline]
    fn is_registered(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                self.id != Self::INVALID
            } else {
                false
            }
        }
    }
}
impl Default for ValgrindStackId {
    fn default() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                Self { id: Self::INVALID }
            } else {
                Self {}
            }
        }
    }
}

/// A coroutine stack
pub struct Stack {
    /// The total size of the stack (including guarded pages)
    total_size: usize,
    /// The start end of the stack to use as a coroutine stack
    base: *mut u8,
    /// The valdring stack identifier (When running tests under valgrind)
    valgrind_stack_id: ValgrindStackId,
}

impl Drop for Stack {
    fn drop(&mut self) {
        self.valgrind_stack_id.deregister();
        stack_dealloc(self.total_size, Self::guard_size(), unsafe {
            NonNull::new_unchecked(self.base)
        });
    }
}

impl Stack {
    pub const DEFAULT_TOTAL_SIZE: usize = 65536 * std::mem::size_of::<usize>();

    /// Returns the system page allocation granularity
    #[inline(always)]
    pub fn page_size() -> usize {
        page_size::get_granularity()
    }

    /// Returns the guard size
    #[inline(always)]
    pub fn guard_size() -> usize {
        page_size::get_granularity()
    }

    /// Returns a new stack with the given `size_hint` or `None` when the system is out of memory
    pub fn with_size(mut size_hint: usize) -> Option<Self> {
        let guard_size = Self::guard_size();
        let page_align_mask = Self::page_size() - 1;
        size_hint += guard_size + (size_hint == 0) as usize;
        size_hint = (size_hint + page_align_mask) & !page_align_mask;
        if let Some(base) = stack_alloc(size_hint, guard_size) {
            Some(Self {
                total_size: size_hint,
                base: base.as_ptr(),
                valgrind_stack_id: ValgrindStackId::default(),
            })
        } else {
            None
        }
    }

    /// Returns a new stack with the default size or `None` when the system is out of memory
    pub fn new() -> Option<Stack> {
        Self::with_size(Self::DEFAULT_TOTAL_SIZE - Self::guard_size())
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(target_arch="x86", target_arch="x86_64", target_arch="aarch64"))] {
        /// Returns `true` when the stack growth downward
        #[inline(always)]
        pub const fn stack_growth_downward() -> bool {
            true
        }
    } else {
        #[inline(never)]
        fn __get_stack_growth_downward(prev_stack_data: *mut u8) -> bool {
            let mut data_on_stack = 0u8;
            (&mut data_on_stack as *mut u8 as usize) < (prev_stack_data as usize)
        }
        /// Returns `true` when the stack growth downward
        #[inline(never)]
        pub fn stack_growth_downward() -> bool {
            let mut data_on_stack = 0u8;
            __get_stack_growth_downward(&mut data_on_stack as _)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page() {
        let stack = Stack::with_size(0).unwrap();
        assert_eq!(stack.total_size, Stack::page_size() + Stack::guard_size());

        let stack = Stack::with_size(Stack::page_size() - 1).unwrap();
        assert_eq!(stack.total_size, Stack::page_size() + Stack::guard_size());

        let stack = Stack::with_size(Stack::page_size()).unwrap();
        assert_eq!(stack.total_size, Stack::page_size() + Stack::guard_size());

        let stack = Stack::with_size(Stack::page_size() * 4).unwrap();
        assert_eq!(
            stack.total_size,
            Stack::page_size() + Stack::guard_size() * 4
        );

        let stack = Stack::new().unwrap();
        assert_eq!(stack.total_size, Stack::DEFAULT_TOTAL_SIZE);
    }
}
