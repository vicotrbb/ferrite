# 2026-06-27 Q5_0 Retained Matrix Slice

## Scope

This slice keeps Q5_0 GGUF matrices in compressed byte storage inside the
scalar model instead of expanding them into owned F32 matrices at load time.

## Implementation

- Added Q5_0-backed storage to `Matrix`.
- Added `Matrix::from_q5_0_row_major_bytes` for validated row-major Q5_0
  matrix construction.
- Added Q5_0 row decoding for scalar execution through `Matrix::row_values`.
- Updated the GGUF scalar loader to retain Q5_0 matrix bytes.
- Preserved F32 decoding for norm vectors and non-matrix scalar data.

## Boundaries

This is still a scalar reference memory slice. Q5_0 rows are decoded during
scalar matrix-vector multiplication, so the implementation trades retained
memory for more decode work. Q4_K and Q6_K matrices still expand to F32 in this
slice.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q5_0_gguf_fixture` failed because
  the Q5 fixture reported `scalar_weight_bytes=29568` instead of `5400`.
- Green: the same targeted test passed after retaining Q5_0 matrix bytes.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
