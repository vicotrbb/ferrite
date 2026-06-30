# 2026-06-30 GGUF Model Config KV Head Count Guardrail

## Summary

Ferrite now rejects explicit zero `*.attention.head_count_kv` metadata while
deriving a model config.

The metadata field is optional and still defaults to `attention.head_count`
when absent. When present, zero KV heads describe an invalid attention layout
and should fail at the model-config boundary instead of leaking into later
session or GQA behavior.

## Changes

- Added optional non-zero count validation for GGUF model config metadata.
- Applied it to `{architecture}.attention.head_count_kv`.
- Extended the minimal Llama GGUF fixture options helper to vary KV head count.
- Added a regression test for explicit zero KV attention head count.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_reader rejects_zero_attention_kv_head_count_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero KV attention head count should be rejected" }
test rejects_zero_attention_kv_head_count_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_reader rejects_zero_attention_kv_head_count_in_model_config -- --nocapture
test rejects_zero_attention_kv_head_count_in_model_config ... ok
```
