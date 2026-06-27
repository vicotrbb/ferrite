# 2026-06-27 F16 and BF16 Unquantized Loader Slice

## Slice

Advance the GGUF-to-scalar loading path from F32-only generated fixtures to
unquantized F32/F16/BF16 tensor fixtures. This matters for Tier 0 bring-up
because small GGUF models may contain half-precision tensors even before
quantized kernels exist.

## Implementation

- Added a generated GGUF F16 fixture to the scalar reference tests.
- Added safe IEEE-754 half-precision decoding into `f32`.
- Added a generated GGUF BF16 fixture to the scalar reference tests.
- Added safe bfloat16 decoding into `f32`.
- Updated scalar tensor decoding to accept GGML `F32`, `F16`, and `BF16`.
- Added `ScalarLlamaModel::from_gguf_unquantized`.
- Kept `ScalarLlamaModel::from_gguf_f32` as a compatibility wrapper.

## Validation

TDD red step:

```text
cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_f16_gguf_fixture
Error: InferenceError { message: "tensor blk.0.attn_norm.weight has type F16; expected F32" }
```

Green step:

```text
cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_f16_gguf_fixture
test result: ok. 1 passed; 0 failed
```

BF16 red step:

```text
cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_bf16_gguf_fixture
Error: InferenceError { message: "tensor blk.0.attn_norm.weight has type BF16; expected F32 or F16" }
```

BF16 green step:

```text
cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_bf16_gguf_fixture
test result: ok. 1 passed; 0 failed
```

## Remaining Unproven

- Quantized GGML tensor formats are not implemented.
- The half-precision fixtures are generated and synthetic, not a real Tier 0
  model.
- Tokenization and llama.cpp reference comparison remain missing.

## Next Slice

Add tokenizer metadata extraction and a minimal tokenizer boundary, or start
scalar dequantization for the first real Tier 0 GGUF quantization type.
