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
- Loops whose count comes from a wrapped length are capped (`LOOP_CAP`,
  `max_steps`); real code would continue.
- `Arith::Checked` means "stop at the first overflow". Whether a given std
  build had overflow checks is not claimed here.
- Affected range "before 1.52.0" is from the GHSA record; no toolchain was
  run to confirm it.
- CI configuration is included; at the time of the local commit it had only
  been run locally, not on a hosted runner.
