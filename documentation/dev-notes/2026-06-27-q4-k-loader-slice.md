# 2026-06-27 Q4_K Loader Slice

## Scope

This slice adds fixture-validated GGML `Q4_K` tensor decoding for the scalar
reference loader.

## Implementation

- Added `scalar_llama_q4_k_gguf_fixture` to `ferrite-fixtures`.
- The fixture uses Q4_K for matrix tensors and F32 for normalization vectors.
- Implemented Q4_K dequantization in `scalar/tensor.rs`.
- The decoder follows the upstream `ggml` layout: 256 values per 144-byte
  block, two F16 super-scales, 12 packed scale/min bytes, and 128 packed
  4-bit quant bytes.

## Boundaries

This is scalar reference dequantization into F32 values. It does not implement
Q4_K fused matmul, Q4_K_M file-type policy, imatrix handling, mmap streaming, or
real Tier 0 model parity.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q4_k_gguf_fixture` failed because
  the scalar tensor decoder rejected `Q4K`.
- Green: the same targeted test passed after adding Q4_K scalar
  dequantization.
- Layout was refreshed from upstream
  `https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-quants.c`.
