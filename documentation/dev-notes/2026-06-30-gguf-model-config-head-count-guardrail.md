# 2026-06-30 GGUF Model Config Head Count Guardrail

## Summary

Ferrite now rejects zero `*.attention.head_count` metadata while deriving a
model config instead of panicking during default key/value head-dimension
calculation.

The config loader used `unwrap_or(embedding_length / attention_head_count)` for
optional key and value lengths. `unwrap_or` evaluates its argument eagerly, so a
malformed file with explicit key/value lengths and a zero attention head count
could still divide by zero.

## Changes

- Added a required non-zero count helper for GGUF model config metadata.
- Applied it to `{architecture}.attention.head_count`.
- Reused a single minimal Llama GGUF fixture builder for offset and head-count
  variants.
- Added a regression test for zero attention head count.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_reader rejects_zero_attention_head_count_in_model_config -- --nocapture
thread 'rejects_zero_attention_head_count_in_model_config' panicked at crates/ferrite-model/src/gguf.rs:72:24:
attempt to divide by zero
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_reader rejects_zero_attention_head_count_in_model_config -- --nocapture
test rejects_zero_attention_head_count_in_model_config ... ok
```
