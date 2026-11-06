# cortex9-rust-zip-size-hint-overflow-model

[Español](README.es.md) · [Practice guide](PRACTICE.md)

A safe-Rust model (`#![forbid(unsafe_code)]`) of the `Zip` iterator's
`TrustedRandomAccess` specialisation in `core`, before and after
rust-lang/rust PR #82289, which fixed CVE-2021-28879.

**This is a model, not the standard library and not a reproduction.**
Indices and lengths are model values in a chosen word size (8/16/32/64
bits). `__iterator_get_unchecked` is replaced by a recorded event that is
classified as in/out of contract and in/out of the backing allocation;
nothing is ever read out of bounds and no old or vulnerable toolchain is
used. See [LIMITATIONS.md](LIMITATIONS.md).

## The mechanism, as ported

`library/core/src/iter/adapters/zip.rs` at 0148b971c921 (specialised impl):

- `next()`, side-effect branch (`index >= len` but `A` may have side
  effects and `index < a.size()`): `self.index += 1` **without**
  `self.len += 1`, so `index > len` becomes possible.
- `size_hint()` is `self.len - self.index`: with `index > len` this
  underflows; `TrustedRandomAccess::size()` is `size_hint().0`.
- A `Zip` of such zips takes `len = min(a.size(), b.size())` = the wrapped
  value and then calls `__iterator_get_unchecked(i)` for `i` beyond the real
  data. GHSA-rq3p-62m7-c66f: "incorrect size due to an integer overflow ...
  can lead to a buffer overflow when a consumed Zip iterator is used again."
- PR #82289 adds `self.len += 1` in that branch and changes `next_back`'s
  `sz_a - cmp::max(self.len, self.index)` to `sz_a - self.len`. It adds
  `test_issue_82282`, ported here as scenario 01.

| Model | Source |
|---|---|
| `zip::Zip::{new, next, nth, next_back}`, `It::size` | zip.rs at 0148b97 (`Variant::Vulnerable`) and PR #82289 diff (`Variant::Fixed`) |
| scenario 01 | `test_issue_82282` in the PR |
| `word::Arith::{Wrapping, Checked}` | wrap vs. stop at the first overflow |

## Run

```sh
cargo test
cargo run --release --bin zip-model -- run fixtures/scenarios/01_issue_82282.json
cargo run --release --bin zip-model -- matrix fixtures/scenarios /tmp/out
diff -r results /tmp/out
```

## Results (from `results/matrix.csv`, 6 scenarios × 2 variants × 2 arith × 3 word sizes)

- 01, vulnerable + wrapping: outer `size_hint` = 2^W − 1 for W = 16/32/64;
  the 5 capped steps make 22 `get_unchecked` calls, 16 outside the backing
  allocation (`out_of_allocation_zst` = 10 on the zero-sized `[(); 0]`, 6 on
  the `i32` array), all counted, none performed.
- 01, vulnerable + checked: stops at the first overflow (`0 - 1`).
- 01, fixed: `size_hint` 0, no items, matching the upstream test.
- 03 / 06: a drained `map(a).zip(b)` with `a > b`, polled again once / twice,
  reports `size_hint` 2^W − 1 / 2^W − 3 (05, via `nth`: 2^W − 3 with two
  overflows). No out-of-bounds call by itself, but a wrong size that a
  wrapping outer `Zip` then trusts (01). Fixed variant: 0.
- 02 (benign) is identical in both variants. Fixed variant: no overflow, no
  out-of-contract or out-of-allocation call in any scenario.

Licence: Apache-2.0.
