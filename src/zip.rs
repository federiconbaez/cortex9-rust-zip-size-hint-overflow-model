use crate::word::{Arith, Word};
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// zip.rs at 0148b971c921 (before PR #82289).
    Vulnerable,
    /// After PR #82289: `self.len += 1` in the side-effect branch of `next`,
    /// and `sz_a - self.len` in `next_back`.
    Fixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Overflow {
        site: String,
        op: String,
        a: u64,
        b: u64,
        result: u64,
    },
    GetUnchecked {
        iter: String,
        idx: u64,
        size: u64,
        alloc_len: u64,
        in_contract: bool,
        in_allocation: bool,
    },
    NextBack {
        iter: String,
    },
}

/// Upper bound on model loops whose count comes from a (possibly wrapped)
/// length; real code would loop that many times.
pub const LOOP_CAP: u64 = 1 << 16;

/// Raised when `Arith::Checked` meets an overflow (a panic in real code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Halt;

pub struct Ctx {
    pub word: Word,
    pub arith: Arith,
    pub variant: Variant,
    pub events: Vec<Event>,
}

impl Ctx {
    pub fn new(bits: u32, arith: Arith, variant: Variant) -> Self {
        Ctx {
            word: Word::new(bits),
            arith,
            variant,
            events: vec![],
        }
    }

    fn sub(&mut self, site: &str, a: u64, b: u64) -> Result<u64, Halt> {
        let (r, o) = self.word.sub(a, b);
        if o {
            self.events.push(Event::Overflow {
                site: site.into(),
                op: "sub".into(),
                a,
                b,
                result: r,
            });
            if self.arith == Arith::Checked {
                return Err(Halt);
            }
        }
        Ok(r)
    }

    fn add(&mut self, site: &str, a: u64, b: u64) -> Result<u64, Halt> {
        let (r, o) = self.word.add(a, b);
        if o {
            self.events.push(Event::Overflow {
                site: site.into(),
                op: "add".into(),
                a,
                b,
                result: r,
            });
            if self.arith == Arith::Checked {
                return Err(Halt);
            }
        }
        Ok(r)
    }
}

/// A slice iterator (optionally behind a side-effecting `map`).
#[derive(Debug, Clone)]
pub struct Slice {
    pub name: String,
    /// Length of the backing allocation.
    pub alloc_len: u64,
    pub front: u64,
    pub back: u64,
    /// `may_have_side_effect()`: true for `map(..)`, false for plain `iter()`.
    pub side_effect: bool,
}

#[derive(Debug, Clone)]
pub struct Zip {
    pub name: String,
    pub a: Box<It>,
    pub b: Box<It>,
    pub index: u64,
    pub len: u64,
}

#[derive(Debug, Clone)]
pub enum It {
    Slice(Slice),
    Zip(Zip),
}

impl It {
    pub fn slice(name: &str, len: u64, side_effect: bool) -> It {
        It::Slice(Slice {
            name: name.into(),
            alloc_len: len,
            front: 0,
            back: len,
            side_effect,
        })
    }

    pub fn name(&self) -> &str {
        match self {
            It::Slice(s) => &s.name,
            It::Zip(z) => &z.name,
        }
    }

    pub fn may_have_side_effect(&self) -> bool {
        match self {
            It::Slice(s) => s.side_effect,
            It::Zip(z) => z.a.may_have_side_effect() || z.b.may_have_side_effect(),
        }
    }

    /// `size_hint().0`; `TrustedRandomAccess::size()` is `size_hint().0`.
    pub fn size(&self, cx: &mut Ctx) -> Result<u64, Halt> {
        match self {
            It::Slice(s) => Ok(s.back - s.front),
            It::Zip(z) => cx.sub("Zip::size_hint: self.len - self.index", z.len, z.index),
        }
    }

    /// Model of `__iterator_get_unchecked(idx)`: never reads memory, only
    /// records whether the call respects the contract and the allocation.
    pub fn get_unchecked(&mut self, cx: &mut Ctx, idx: u64) -> Result<(), Halt> {
        match self {
            It::Slice(s) => {
                let size = s.back - s.front;
                let (abs, o) = cx.word.add(s.front, idx);
                cx.events.push(Event::GetUnchecked {
                    iter: s.name.clone(),
                    idx,
                    size,
                    alloc_len: s.alloc_len,
                    in_contract: idx < size,
                    in_allocation: !o && abs < s.alloc_len,
                });
                Ok(())
            }
            It::Zip(z) => {
                let i = cx.add("Zip::get_unchecked: self.index + idx", z.index, idx)?;
                z.a.get_unchecked(cx, i)?;
                z.b.get_unchecked(cx, i)
            }
        }
    }

    pub fn next_back(&mut self, cx: &mut Ctx) -> Result<bool, Halt> {
        match self {
            It::Slice(s) => {
                cx.events.push(Event::NextBack {
                    iter: s.name.clone(),
                });
                if s.back > s.front {
                    s.back -= 1;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            It::Zip(z) => z.next_back(cx),
        }
    }

    pub fn next(&mut self, cx: &mut Ctx) -> Result<bool, Halt> {
        match self {
            It::Slice(s) => {
                if s.front < s.back {
                    s.front += 1;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            It::Zip(z) => z.next(cx),
        }
    }
}

impl Zip {
    /// `ZipImpl::new` (specialised): `len = min(a.size(), b.size())`.
    pub fn new(cx: &mut Ctx, name: &str, a: It, b: It) -> Result<Zip, Halt> {
        let len = cmp::min(a.size(cx)?, b.size(cx)?);
        Ok(Zip {
            name: name.into(),
            a: Box::new(a),
            b: Box::new(b),
            index: 0,
            len,
        })
    }

    pub fn next(&mut self, cx: &mut Ctx) -> Result<bool, Halt> {
        if self.index < self.len {
            let i = self.index;
            self.index = cx.add("Zip::next: self.index += 1", self.index, 1)?;
            self.a.get_unchecked(cx, i)?;
            self.b.get_unchecked(cx, i)?;
            Ok(true)
        } else if self.a.may_have_side_effect() && self.index < self.a.size(cx)? {
            let i = self.index;
            self.index = cx.add("Zip::next: self.index += 1", self.index, 1)?;
            if cx.variant == Variant::Fixed {
                self.len = cx.add("Zip::next: self.len += 1", self.len, 1)?;
            }
            // match the base implementation's potential side effects
            self.a.get_unchecked(cx, i)?;
            Ok(false)
        } else {
            Ok(false)
        }
    }

    pub fn nth(&mut self, cx: &mut Ctx, n: u64) -> Result<bool, Halt> {
        let remaining = cx.sub("Zip::nth: self.len - self.index", self.len, self.index)?;
        let delta = cmp::min(n, remaining);
        let end = cx.add("Zip::nth: self.index + delta", self.index, delta)?;
        while self.index < end {
            let i = self.index;
            self.index = cx.add("Zip::nth: self.index += 1", self.index, 1)?;
            if self.a.may_have_side_effect() {
                self.a.get_unchecked(cx, i)?;
            }
            if self.b.may_have_side_effect() {
                self.b.get_unchecked(cx, i)?;
            }
        }
        // super_nth(n - delta): default nth = advance_by(n - delta) then next()
        let rest = cx.sub("Zip::nth: n - delta", n, delta)?;
        let mut last = false;
        for _ in 0..=rest.min(LOOP_CAP) {
            last = self.next(cx)?;
        }
        Ok(last)
    }

    pub fn next_back(&mut self, cx: &mut Ctx) -> Result<bool, Halt> {
        let a_side = self.a.may_have_side_effect();
        let b_side = self.b.may_have_side_effect();
        if a_side || b_side {
            let sz_a = self.a.size(cx)?;
            let sz_b = self.b.size(cx)?;
            if sz_a != sz_b {
                let sz_a = self.a.size(cx)?;
                if a_side && sz_a > self.len {
                    let n = match cx.variant {
                        Variant::Vulnerable => cx.sub(
                            "Zip::next_back: sz_a - max(len, index)",
                            sz_a,
                            cmp::max(self.len, self.index),
                        )?,
                        Variant::Fixed => cx.sub("Zip::next_back: sz_a - len", sz_a, self.len)?,
                    };
                    for _ in 0..n.min(LOOP_CAP) {
                        self.a.next_back(cx)?;
                    }
                }
                let sz_b = self.b.size(cx)?;
                if b_side && sz_b > self.len {
                    let n = cx.sub("Zip::next_back: sz_b - len", sz_b, self.len)?;
                    for _ in 0..n.min(LOOP_CAP) {
                        self.b.next_back(cx)?;
                    }
                }
            }
        }
        if self.index < self.len {
            self.len -= 1;
            let i = self.len;
            self.a.get_unchecked(cx, i)?;
            self.b.get_unchecked(cx, i)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
