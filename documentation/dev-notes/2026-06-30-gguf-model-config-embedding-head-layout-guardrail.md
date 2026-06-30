# 2026-06-30 GGUF Model Config Embedding Head Layout Guardrail

## Summary

Ferrite now rejects GGUF model configs where `*.embedding_length` is not
divisible by `*.attention.head_count`.

The scalar loader derives the default per-head dimension from
`embedding_length / attention.head_count`. If that division has a remainder, the
model metadata would silently truncate the head width and produce an invalid
attention layout.

## Changes

- Added a model-config layout validator requiring
  `{architecture}.embedding_length % {architecture}.attention.head_count == 0`.
- Added a regression test for a non-divisible embedding/head layout.
- Adjusted the attention-head fixture variant to keep embedding/head layout
  valid when testing the separate KV-head divisibility invariant.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_embedding_length_that_does_not_divide_attention_heads -- --nocapture
Error: Custom { kind: Other, error: "embedding length that does not divide attention heads should be rejected" }
test rejects_embedding_length_that_does_not_divide_attention_heads ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_embedding_length_that_does_not_divide_attention_heads -- --nocapture
test rejects_embedding_length_that_does_not_divide_attention_heads ... ok
```
