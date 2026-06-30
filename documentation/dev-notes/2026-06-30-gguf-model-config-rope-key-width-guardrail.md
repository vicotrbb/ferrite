# 2026-06-30 GGUF Model Config RoPE Key Width Guardrail

## Summary

Ferrite now rejects `*.rope.dimension_count` values that exceed
`*.attention.key_length` while deriving a GGUF model config.

The scalar RoPE path rotates dimensions inside each key head. A RoPE width
larger than the key head width is malformed and would let invalid metadata reach
attention execution.

## Changes

- Added a model-config layout validator requiring
  `{architecture}.rope.dimension_count <= {architecture}.attention.key_length`.
- Added a regression test for a RoPE dimension count larger than key length.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_rope_dimension_count_larger_than_key_length -- --nocapture
Error: Custom { kind: Other, error: "rope dimension count larger than key length should be rejected" }
test rejects_rope_dimension_count_larger_than_key_length ... FAILED
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_config rejects_rope_dimension_count_larger_than_key_length -- --nocapture
test rejects_rope_dimension_count_larger_than_key_length ... ok
```
