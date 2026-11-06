# Limitations

- **Model, not `core`.** Only the specialised `ZipImpl` paths listed in the
  README are ported. The general (non-specialised) `Zip`, `fold`/`try_fold`
  and other adapters are not modelled.
- **No memory is touched.** `out_of_allocation_prevented` counts calls that
  would have indexed past the backing allocation in real code. For the
  zero-sized `[(); 0]` slice such an index reads no bytes; those are
  reported separately as `out_of_allocation_zst`. Exploitability is not
  assessed.
- `Slice` stands for `slice::Iter` (optionally behind `map`); its
  `get_unchecked` is relative to the current front, which `Zip` never
  advances in these paths.
- **Loop limit.** A loop whose exact iteration count exceeds `LOOP_CAP`
  (65536), or the `for` loop of scenario 01 beyond `max_steps`, is **not**
  run: the run ends with `status = ModelLimit`, the site and the requested
  count are reported, and `counts_partial = true`. Counters then cover only
  what ran before; what real code would do afterwards (including any
  further out-of-bounds calls or the final `size_hint`) is indeterminate and
  not claimed. Scenario 01's out-of-allocation counts are for 5 steps only.
- With 16-bit words no representable `nth`/`next_back` input exceeds the
  cap; inputs wider than the word are reported as `NotRepresentable`.
- `Arith::Checked` means "stop at the first overflow". Whether a given std
  build had overflow checks is not claimed here.
- Affected range "before 1.52.0" is from the GHSA record; no toolchain was
  run to confirm it.
- CI is defined in `.github/workflows/ci.yml` (fmt, clippy, tests, results regeneration). Results were also
  reproduced locally with rustc 1.99.1; the status of hosted runs is whatever the
  repository's Actions page shows.
