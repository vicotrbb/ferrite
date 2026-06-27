# 2026-06-27 Scalar Math Module Split Slice

## Scope

This slice continues reducing `scalar.rs` by extracting scalar math helpers into
a focused module.

## Implementation

- Added `crates/ferrite-inference/src/scalar/math.rs`.
- Re-exported public helpers `rms_norm`, `argmax`, and `apply_rope` from
  `ferrite_inference::scalar`.
- Kept internal helpers such as `softmax`, `swiglu`, `dot`, `add_assign`, and
  `ensure_len` scoped to the scalar module.

## Evidence Plan

This is a behavior-preserving refactor. The existing scalar reference tests and
workspace tests remain the verification gate.
