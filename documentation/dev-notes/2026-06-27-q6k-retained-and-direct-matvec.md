# 2026-06-27 Q6_K Retained Matrix and Direct Matvec

## Scope

This pair of slices completes retained scalar matrix storage for the remaining
`Q6_K` tensors in the SmolLM2 Q4_K_M GGUF path, then fixes the immediate
row-by-row decode latency regression.

## Implementation

- Added Q6_K fixture writer support in `ferrite-fixtures`.
- Added a Q6_K scalar Llama fixture using the existing Q4_K-sized matrix
  shape so each quantized matrix is block-aligned.
- Added shared Q6_K storage-byte and decode helpers in
  `crates/ferrite-inference/src/scalar/quantized.rs`.
- Updated tensor-level Q6_K dequantization to reuse the shared quantized
  decoder.
- Added Q6_K-backed storage to `Matrix`.
- Updated the GGUF scalar loader to retain Q6_K matrix bytes.
- Added `q6_k_mul_vec` and routed Q6_K `Matrix::mul_vec` calls through it.

## Boundaries

This is still scalar reference execution. Q6_K matrices are decoded during each
matrix-vector multiply into scalar values before accumulation. The direct
matvec path avoids decoding the whole matrix once per requested row, but it does
not yet implement fused quantized dot products, SIMD, threading, or blocking.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q6_k_gguf_fixture` failed because
  the Q6_K fixture reported `scalar_weight_bytes=117504` instead of `24708`.
- Green: the same targeted test passed after retaining Q6_K matrix bytes.
- Regression probe after commit `9ffc922`: the real SmolLM2 single-token run
  reported `scalar_weight_bytes=103668480` but high wall time, confirming the
  row-by-row decode problem moved to Q6_K.
- Red: `cargo test -p ferrite-inference
  scalar::quantized::tests::q6_k_mul_vec_accumulates_rows_without_full_row_decodes`
  failed because `q6_k_mul_vec` did not exist.
- Green: the same targeted test passed after adding direct Q6_K scalar matvec.
- Full verification before each code commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
