# 2026-06-27 Q8_0 Retained Matrix Slice

## Scope

This slice keeps Q8_0 GGUF matrices in compressed byte storage inside the
scalar model instead of always expanding them into owned F32 matrices at load
time.

## Implementation

- Added Q8_0-backed storage to `Matrix`.
- Added `Matrix::from_q8_0_row_major_bytes` for validated row-major Q8_0
  matrix construction.
- Added `Matrix::row_values` so scalar execution can request F32 row values
  from either F32 or Q8_0 storage.
- Updated scalar matrix-vector multiplication to decode Q8_0 rows on demand.
- Updated the GGUF scalar loader to retain Q8_0 matrix bytes while still
  decoding non-matrix vectors into F32.
- Updated scalar memory accounting to use the matrix's retained storage bytes.

## Boundaries

This is a scalar reference memory slice, not an optimized quantized matmul.
Q8_0 rows are decoded during scalar execution, so decode latency can increase.
Q5_0, Q4_K, and Q6_K matrices still expand to F32 in this slice.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q8_0_gguf_fixture` failed because
  the Q8 fixture reported `scalar_weight_bytes=29568` instead of `8136`.
- Green: the same targeted test passed after retaining Q8_0 matrix bytes.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
