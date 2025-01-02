mod executor;
pub use executor::*;

mod ex2;

use crate::{
    collection::{SLink, SListNode},
    ptr::Ptr,
};
use generator::LocalGenerator;
pub struct Coroutine {
    link: SLink,
    gn: LocalGenerator<'static, usize, usize>,
}
impl SListNode for Coroutine {
    const OFFSET_OF_LINK: usize = std::mem::offset_of!(Coroutine, link);

    // fn new() {
    // }
}
