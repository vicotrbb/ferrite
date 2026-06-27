# 2026-06-27 RoPE and KV Scalar Slice

## Slice

Advance the scalar reference executor toward real Llama-family execution by
adding RoPE rotation and multi-token causal attention over an in-memory K/V
cache. This keeps the first milestone moving from single-token plumbing toward
prompt evaluation.

## Implementation

- Added `rope_dimension_count` and `rope_freq_base` to scalar Llama config.
- Added public scalar `apply_rope`.
- Changed `next_token` to use the same prompt path as multi-token evaluation.
- Added `next_token_for_prompt(&[usize])`.
- Added per-layer K/V accumulation for each prompt position.
- Added causal attention using softmax over cached keys and weighted values.
- Extended GGUF Llama config extraction for:
  - `llama.rope.freq_base`
  - `llama.attention.layer_norm_rms_epsilon`
- Updated the F32 GGUF scalar loader to use those metadata values when present.

## Validation

TDD red step:

```text
cargo test -p ferrite-inference --test scalar_reference
error[E0432]: unresolved import `ferrite_inference::scalar::apply_rope`
error[E0599]: no method named `next_token_for_prompt`
```

Metadata red step:

```text
cargo test -p ferrite-model --test gguf_reader derives_llama_config_from_uint32_or_uint64_metadata
error[E0609]: no field `rope_freq_base` on type `LlamaConfig`
error[E0609]: no field `attention_layer_norm_rms_epsilon` on type `LlamaConfig`
```

Green steps:

```text
cargo test -p ferrite-model --test gguf_reader derives_llama_config_from_uint32_or_uint64_metadata
test result: ok. 1 passed
```

```text
cargo test -p ferrite-inference --test scalar_reference
test result: ok. 6 passed; 0 failed
```

## Remaining Unproven

- The KV cache is rebuilt per prompt call and is not yet an incremental session
  cache.
- RoPE behavior is validated with synthetic scalar fixtures only.
- Real Tier 0 GGUF model loading has not been attempted.
- Tokenization, F16/BF16, quantized tensor decoding, and llama.cpp reference
  comparison remain missing.

## Next Slice

Add a small tokenizer boundary or F16 tensor decoding, then use it with a
downloaded Tier 0 model artifact to begin real model-reference comparison.
