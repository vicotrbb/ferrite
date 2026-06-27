# 2026-06-27 Q6_K Direct Block Accumulation

## Scope

This slice removes the full decoded value-vector allocation inside
`q6_k_mul_vec`. Q6_K scalar matvec now decodes each Q6_K block and immediately
accumulates into output rows.

## Implementation

- Added `accumulate_q6_k_block` in
  `crates/ferrite-inference/src/scalar/quantized.rs`.
- Added a focused unit test that accumulates a synthetic Q6_K block into two
  output rows without materializing a decoded matrix vector.
- Updated `q6_k_mul_vec` to validate storage size, iterate Q6_K blocks, and
  accumulate block values directly.

## Boundaries

This remains scalar reference execution. It removes one allocation-heavy
temporary from Q6_K matvec, but it does not implement fused quantized dot
products, SIMD, threading, or cache blocking.

`Matrix::row_values` still uses full Q6_K matrix decode for direct row
inspection. The inference matvec path no longer does.

## Evidence

- Red: `cargo test -p ferrite-inference
  scalar::quantized::tests::q6_k_block_accumulation_updates_rows_without_decoded_matrix`
  failed because `accumulate_q6_k_block` did not exist.
- Green: `cargo test -p ferrite-inference
  scalar::quantized::tests::q6_k` passed after adding direct block accumulation
  and preserving the existing Q6_K matvec and decoder results.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
