use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arith {
    /// Two's-complement wrap, as `usize` arithmetic without overflow checks.
    Wrapping,
    /// Stop at the first overflow, as with overflow checks enabled.
    Checked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word {
    pub bits: u32,
}

impl Word {
    pub fn new(bits: u32) -> Self {
        assert!(
            matches!(bits, 8 | 16 | 32 | 64),
            "unsupported word size {bits}"
        );
        Word { bits }
    }

    pub fn max(&self) -> u64 {
        if self.bits == 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        }
    }

    /// (wrapped result, overflowed)
    pub fn sub(&self, a: u64, b: u64) -> (u64, bool) {
        let (r, o) = a.overflowing_sub(b);
        (r & self.max(), o)
    }

    pub fn add(&self, a: u64, b: u64) -> (u64, bool) {
        let (r, o) = a.overflowing_add(b);
        let o = o || r > self.max();
        (r & self.max(), o)
    }
}
