# 2026-06-27 Q8_0 Loader Slice

## Scope

This slice adds fixture-validated GGML `Q8_0` tensor decoding for the scalar
reference loader.

## Implementation

- Added `scalar_llama_q8_0_gguf_fixture` to `ferrite-fixtures`.
- Added a scalar loader test covering a generated GGUF fixture whose tensors use
  Q8_0 blocks.
- Extracted tensor value decoding into
  `crates/ferrite-inference/src/scalar/tensor.rs`.
- Implemented Q8_0 dequantization as one F16 scale followed by 32 signed
  quantized values per block.

## Boundaries

This does not implement Q4_0, Q4_K, Q5_K, Q6_K, IQ formats, mmap streaming, or
optimized quantized matrix kernels. Q8_0 values are dequantized into the scalar
F32 reference path.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  loads_scalar_llama_reference_weights_from_q8_0_gguf_fixture` failed because
  the scalar loader rejected `Q8_0`.
- Green: the same targeted test passed after adding the tensor decoder module
  and Q8_0 block dequantization.
