# 2026-06-30 GGUF Model Config RoPE Dimension Guardrail

## Summary

Ferrite now rejects zero explicit `*.rope.dimension_count` metadata while
deriving a GGUF model config.

The scalar RoPE path uses rope dimension count as the rotary width for each
attention head. For Qwen2 metadata that omits the value, Ferrite keeps deriving
the dimension count from the key length. When metadata is present, a zero value
is malformed and is rejected at the model-config boundary.

## Changes

- Applied optional non-zero count validation to
  `{architecture}.rope.dimension_count`.
- Added a minimal fixture helper for Llama RoPE dimension-count variants.
- Added a regression test for zero explicit RoPE dimension count.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_rope_dimension_count_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero rope dimension count should be rejected" }
test rejects_zero_rope_dimension_count_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_rope_dimension_count_in_model_config -- --nocapture
test rejects_zero_rope_dimension_count_in_model_config ... ok
```
