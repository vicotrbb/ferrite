# 2026-06-27 Q4_K Retained Matrix Slice

## Scope

This slice keeps Q4_K GGUF matrices in compressed byte storage inside the
scalar model instead of expanding them into owned F32 matrices at load time.

## Implementation

- Split quantized row decoding helpers into
  `crates/ferrite-inference/src/scalar/quantized.rs`.
- Moved Q5_0 and Q8_0 row decoding out of `matrix.rs`.
- Added Q4_K-backed storage to `Matrix`.
- Added `Matrix::from_q4_k_row_major_bytes` for validated row-major Q4_K
  matrix construction.
- Added Q4_K decode support for scalar execution through `Matrix::row_values`.
- Updated the GGUF scalar loader to retain Q4_K matrix bytes.

## Boundaries

Q4_K is retained as compressed bytes, but this is not yet an efficient Q4_K
matvec. Because Q4_K blocks span 256 contiguous values and do not necessarily
align to a single logical row, the current scalar reference path decodes a
temporary full matrix when a Q4_K row is requested. This preserves retained
model memory but severely increases compute time.

Q6_K matrices still expand to F32 in this slice.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q4_k_gguf_fixture` failed because
  the Q4_K fixture reported `scalar_weight_bytes=117504` instead of `17184`.
- Green: the same targeted test passed after retaining Q4_K matrix bytes.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
