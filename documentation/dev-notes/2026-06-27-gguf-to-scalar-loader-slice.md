# 2026-06-27 GGUF to Scalar Loader Slice

## Slice

Connect the GGUF parser to the scalar reference executor for generated F32
weights. This advances the first milestone from hand-built Rust weights toward
model-file-driven execution.

## Implementation

- Added `ferrite-model` as a dependency of `ferrite-inference`.
- Added `ScalarLlamaModel::from_gguf_f32`.
- Mapped standard GGUF tensor names into scalar Llama weights:
  - `token_embd.weight`
  - `output_norm.weight`
  - `output.weight`
  - `blk.N.attn_norm.weight`
  - `blk.N.attn_q.weight`
  - `blk.N.attn_k.weight`
  - `blk.N.attn_v.weight`
  - `blk.N.attn_output.weight`
  - `blk.N.ffn_norm.weight`
  - `blk.N.ffn_gate.weight`
  - `blk.N.ffn_up.weight`
  - `blk.N.ffn_down.weight`
- Added F32 tensor decoding from validated GGUF tensor byte ranges.
- Added shape checks for GGUF matrix dimensions using `[cols, rows]` order.

## Validation

TDD red step:

```text
cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_f32_gguf_fixture
error[E0599]: no associated function or constant named `from_gguf_f32`
```

Green step:

```text
cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_f32_gguf_fixture
test result: ok. 1 passed; 0 failed; 3 filtered out
```

## Remaining Unproven

- The loaded fixture uses generated F32 weights, not a real Tier 0 model.
- F16, BF16, and quantized tensor decoding are not implemented.
- RoPE and KV cache behavior are not implemented.
- Tokenizer integration is still missing.
- No llama.cpp reference comparison exists yet.

## Next Slice

Add RoPE and a two-token KV cache path to the scalar executor, then validate it
with deterministic synthetic attention fixtures before loading real Tier 0
model artifacts.
