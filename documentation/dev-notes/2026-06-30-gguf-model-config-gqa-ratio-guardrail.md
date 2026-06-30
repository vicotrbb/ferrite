# 2026-06-30 GGUF Model Config GQA Ratio Guardrail

## Summary

Ferrite now rejects model configs where `*.attention.head_count_kv` does not
evenly divide `*.attention.head_count`.

This keeps invalid grouped-query attention metadata from producing a derived
config whose `gqa_ratio()` would be `None`. The model-config boundary now
requires an integer GQA ratio before inference code sees the config.

## Changes

- Added attention head layout validation during GGUF model config derivation.
- Added a minimal fixture helper for combined attention/KV head-count variants.
- Added a regression test for `attention.head_count = 3` and
  `attention.head_count_kv = 2`.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_kv_head_count_that_does_not_divide_attention_heads -- --nocapture
Error: Custom { kind: Other, error: "non-divisible KV attention head count should be rejected" }
test rejects_kv_head_count_that_does_not_divide_attention_heads ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_kv_head_count_that_does_not_divide_attention_heads -- --nocapture
test rejects_kv_head_count_that_does_not_divide_attention_heads ... ok
```
