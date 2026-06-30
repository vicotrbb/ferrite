# 2026-06-30 GGUF Model Config Embedding Length Guardrail

## Summary

Ferrite now rejects zero `*.embedding_length` metadata while deriving a GGUF
model config.

The scalar inference loader treats embedding length as the model hidden size and
requires it to be non-zero before building a session. Rejecting zero hidden size
at the model-config boundary keeps malformed GGUF metadata from progressing to
later inference validation.

## Changes

- Applied required non-zero count validation to `{architecture}.embedding_length`.
- Added a minimal fixture helper for Llama embedding-length variants.
- Added a regression test for zero embedding length.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_embedding_length_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero embedding length should be rejected" }
test rejects_zero_embedding_length_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_embedding_length_in_model_config -- --nocapture
test rejects_zero_embedding_length_in_model_config ... ok
```
