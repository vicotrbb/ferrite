# 2026-06-30 GGUF Model Config Value Length Guardrail

## Summary

Ferrite now rejects zero explicit `*.attention.value_length` metadata while
deriving a GGUF model config.

The scalar attention path uses value length as the per-head value projection
width. When GGUF metadata omits the value length, Ferrite keeps deriving it from
`embedding_length / attention.head_count`. When metadata is present, a zero
value is malformed and is rejected at the model-config boundary.

## Changes

- Applied optional non-zero count validation to
  `{architecture}.attention.value_length`.
- Added a minimal fixture helper for Llama value-length variants.
- Added a regression test for zero explicit value length.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_attention_value_length_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero attention value length should be rejected" }
test rejects_zero_attention_value_length_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_attention_value_length_in_model_config -- --nocapture
test rejects_zero_attention_value_length_in_model_config ... ok
```
