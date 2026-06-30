# 2026-06-30 GGUF Model Config Key Length Guardrail

## Summary

Ferrite now rejects zero explicit `*.attention.key_length` metadata while
deriving a GGUF model config.

The scalar attention path uses key length as the per-head key projection width.
When GGUF metadata omits the key length, Ferrite keeps deriving it from
`embedding_length / attention.head_count`. When metadata is present, a zero
value is malformed and is rejected at the model-config boundary.

## Changes

- Applied optional non-zero count validation to
  `{architecture}.attention.key_length`.
- Added a minimal fixture helper for Llama key-length variants.
- Refactored the Llama GGUF fixture builder around a private options struct so
  metadata variants stay explicit without a long helper argument list.
- Added a regression test for zero explicit key length.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_attention_key_length_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero attention key length should be rejected" }
test rejects_zero_attention_key_length_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_attention_key_length_in_model_config -- --nocapture
test rejects_zero_attention_key_length_in_model_config ... ok
```
