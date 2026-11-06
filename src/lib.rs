//! Safe-Rust model of the `Zip` `TrustedRandomAccess` specialisation in
//! `library/core/src/iter/adapters/zip.rs` before and after rust-lang/rust
//! PR #82289 (CVE-2021-28879).
//!
//! Every index and length is a model value in a parametrised word size.
//! `__iterator_get_unchecked` is replaced by a recorded, bounds-classified
//! event; no unsafe code, no real out-of-bounds access, no old toolchain.
#![forbid(unsafe_code)]

pub mod scenario;
pub mod word;
pub mod zip;
