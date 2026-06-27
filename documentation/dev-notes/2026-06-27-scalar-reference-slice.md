# 2026-06-27 Scalar Reference Slice

## Slice

Advance Ferrite's Tier 0 milestone from parsed model structure toward an
executable scalar forward path. This slice creates a synthetic, one-token,
one-layer Llama-shaped reference path that produces deterministic logits and a
next-token argmax.

## Implementation

- Added `crates/ferrite-inference`.
- Added `ferrite_inference::scalar` with:
  - Shape-checked row-major `Matrix`.
  - Scalar `rms_norm`.
  - Scalar matrix-vector multiply.
  - Single-token GQA attention behavior.
  - SwiGLU feed-forward path.
  - Final output projection and deterministic `argmax`.
  - Model and weight validation for the supported scalar fixture shape.

## Validation

TDD red step:

```text
cargo test -p ferrite-inference --test scalar_reference
error[E0583]: file not found for module `scalar`
```

Green step:

```text
cargo test -p ferrite-inference --test scalar_reference
test result: ok. 3 passed; 0 failed
```

## Remaining Unproven

- The scalar path is synthetic and single-token only.
- RoPE is not implemented.
- Multi-token causal attention and KV cache updates are not implemented.
- GGUF tensor data is not yet connected to scalar weights.
- No tokenizer exists.
- No real Tier 0 model has been loaded or compared with llama.cpp output.
- No performance benchmark is relevant yet.

## Next Slice

Connect `ferrite-model` tensor ranges to scalar F32 tensor decoding for a
generated GGUF fixture, then use that loaded fixture to instantiate the scalar
Llama reference model instead of hand-built Rust weights.
