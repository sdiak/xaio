use crate::OpCode;

const USIZE_BIT_ROT: usize = (std::mem::size_of::<usize>() * 8).ilog2() as _;
const NUM_USIZE_MAX: usize = 256 >> USIZE_BIT_ROT;
const OP_CODE_MASK: u8 = (1u8 << USIZE_BIT_ROT) - 1;

pub struct OpCodeSet {
    bits: [usize; NUM_USIZE_MAX],
}
impl OpCodeSet {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            bits: unsafe { std::mem::zeroed() },
        }
    }
    pub const fn new(op_codes: &[OpCode]) -> Self {
        let mut opset = Self::empty();
        let mut i = 0usize;
        while i < op_codes.len() {
            opset.set_u8(op_codes[i] as u8);
            i += 1;
        }
        opset
    }
    #[inline(always)]
    pub const fn set(&mut self, op_code: OpCode) {
        self.set_u8(op_code as u8);
    }
    #[inline(always)]
    pub const fn clr(&mut self, op_code: OpCode) {
        self.clr_u8(op_code as u8);
    }
    #[inline(always)]
    pub const fn get(&mut self, op_code: OpCode) -> bool {
        self.get_u8(op_code as u8)
    }
    #[inline(always)]
    pub const fn set_u8(&mut self, op_code: u8) {
        self.bits[(op_code as usize) >> USIZE_BIT_ROT] |= 1usize << (op_code & OP_CODE_MASK);
    }
    #[inline(always)]
    pub const fn clr_u8(&mut self, op_code: u8) {
        self.bits[(op_code as usize) >> USIZE_BIT_ROT] &= !(1usize << (op_code & OP_CODE_MASK));
    }
    #[inline(always)]
    pub const fn get_u8(&mut self, op_code: u8) -> bool {
        (self.bits[(op_code as usize) >> USIZE_BIT_ROT] & (1usize << (op_code & OP_CODE_MASK))) != 0
    }
}
