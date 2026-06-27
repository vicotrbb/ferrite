# 2026-06-27 Q4_K Direct Matvec Slice

## Scope

This slice fixes the worst latency regression introduced by retained Q4_K
matrix storage. The previous Q4_K retained path decoded a temporary full matrix
for each requested row. This slice decodes each Q4_K matrix once per scalar
matrix-vector multiply and accumulates row outputs directly.

## Implementation

- Added `q4_k_mul_vec` in `crates/ferrite-inference/src/scalar/quantized.rs`.
- Added a focused unit test using a synthetic Q4_K block of ones.
- Updated `Matrix::mul_vec` to route Q4_K matrices through the direct scalar
  matvec helper.

## Boundaries

This is still not an optimized production quantized matmul. It decodes Q4_K
blocks into scalar values during each matrix-vector multiply. It avoids the
much worse row-by-row full-matrix decode pattern but does not yet exploit SIMD,
threading, blocking, or fused quantized dot products.

## Evidence

- Red: `cargo test -p ferrite-inference
  scalar::quantized::tests::q4_k_mul_vec_accumulates_rows_without_full_row_decodes`
  failed because `q4_k_mul_vec` did not exist.
- Green: the same targeted test passed after adding direct Q4_K scalar matvec.
- Regression check: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q4_k_gguf_fixture` passed.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
