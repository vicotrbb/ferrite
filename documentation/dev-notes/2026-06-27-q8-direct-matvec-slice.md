# 2026-06-27 Q8_0 Direct Matvec Slice

## Scope

This slice routes retained `Q8_0` matrices through a direct scalar
matrix-vector multiply helper instead of the generic `Matrix::mul_vec` fallback
that decodes one temporary row vector at a time.

## Implementation

- Added `q8_0_mul_vec` in `crates/ferrite-inference/src/scalar/quantized.rs`.
- Added a focused unit test using two synthetic Q8_0 rows with different
  quantized values.
- Updated `Matrix::mul_vec` to dispatch Q8_0 matrices to the direct helper.

## Boundaries

This is still scalar reference execution. The direct helper removes per-row
decoded vector allocation during Q8_0 matvec, but it does not implement SIMD,
threading, cache blocking, or fused quantized kernels.

Q5_0 matrices still use the row-decode plus scalar dot fallback.

## Evidence

- Red: `cargo test -p ferrite-inference
  scalar::quantized::tests::q8_0_mul_vec_accumulates_rows_without_row_decodes`
  failed because `q8_0_mul_vec` did not exist.
- Green: the same targeted test passed after adding direct Q8_0 scalar matvec.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
