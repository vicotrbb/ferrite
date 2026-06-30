# 2026-06-30 GGUF Model Config Context Length Guardrail

## Summary

Ferrite now rejects zero `*.context_length` metadata while deriving a GGUF
model config.

The scalar session uses context length as the model's bounded sequence capacity.
Rejecting a zero context window at the model-config boundary keeps malformed
GGUF metadata from reaching KV-cache and prompt execution paths.

## Changes

- Applied required non-zero count validation to `{architecture}.context_length`.
- Added a minimal fixture helper for Llama context-length variants.
- Added a regression test for zero context length.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_context_length_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero context length should be rejected" }
test rejects_zero_context_length_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_context_length_in_model_config -- --nocapture
test rejects_zero_context_length_in_model_config ... ok
```
