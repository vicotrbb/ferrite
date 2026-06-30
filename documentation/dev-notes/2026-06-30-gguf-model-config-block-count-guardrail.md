# 2026-06-30 GGUF Model Config Block Count Guardrail

## Summary

Ferrite now rejects zero `*.block_count` metadata while deriving a GGUF model
config.

The scalar inference path treats block count as the transformer layer count.
Rejecting a zero layer count at the model-config boundary keeps malformed GGUF
metadata from constructing an invalid layer stack.

## Changes

- Applied required non-zero count validation to `{architecture}.block_count`.
- Added a minimal fixture helper for Llama block-count variants.
- Added a regression test for zero block count.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_block_count_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero block count should be rejected" }
test rejects_zero_block_count_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_block_count_in_model_config -- --nocapture
test rejects_zero_block_count_in_model_config ... ok
```
