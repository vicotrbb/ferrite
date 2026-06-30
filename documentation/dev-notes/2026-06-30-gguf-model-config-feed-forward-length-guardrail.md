# 2026-06-30 GGUF Model Config Feed-Forward Length Guardrail

## Summary

Ferrite now rejects zero `*.feed_forward_length` metadata while deriving a GGUF
model config.

The scalar inference path uses feed-forward length as the hidden width of each
transformer block's FFN. Rejecting a zero FFN width at the model-config boundary
keeps malformed GGUF metadata from constructing invalid feed-forward matrices.

## Changes

- Applied required non-zero count validation to
  `{architecture}.feed_forward_length`.
- Added a minimal fixture helper for Llama feed-forward-length variants.
- Added a regression test for zero feed-forward length.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_feed_forward_length_in_model_config -- --nocapture
Error: Custom { kind: Other, error: "zero feed-forward length should be rejected" }
test rejects_zero_feed_forward_length_in_model_config ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_zero_feed_forward_length_in_model_config -- --nocapture
test rejects_zero_feed_forward_length_in_model_config ... ok
```
