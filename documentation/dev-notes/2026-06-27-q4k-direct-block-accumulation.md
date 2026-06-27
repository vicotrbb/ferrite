# 2026-06-27 Q4_K Direct Block Accumulation

## Scope

This slice removes the full decoded value-vector allocation inside
`q4_k_mul_vec`. Q4_K scalar matvec now decodes each Q4_K block and immediately
accumulates into output rows.

## Implementation

- Added `accumulate_q4_k_block` in
  `crates/ferrite-inference/src/scalar/quantized.rs`.
- Added a focused unit test that accumulates a synthetic Q4_K block into two
  output rows without materializing a decoded matrix vector.
- Updated `q4_k_mul_vec` to validate storage size, iterate Q4_K blocks, and
  accumulate block values directly.

## Boundaries

This remains scalar reference execution. It removes one allocation-heavy
temporary from Q4_K matvec, but it does not implement fused quantized dot
products, SIMD, threading, or cache blocking.

`Matrix::row_values` still uses full Q4_K matrix decode for direct row
inspection. The inference matvec path no longer does.

## Evidence

- Red: `cargo test -p ferrite-inference
  scalar::quantized::tests::q4_k_block_accumulation_updates_rows_without_decoded_matrix`
  failed because `accumulate_q4_k_block` did not exist.
- Green: `cargo test -p ferrite-inference
  scalar::quantized::tests::q4_k` passed after adding direct block
  accumulation and preserving the existing Q4_K matvec result.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
