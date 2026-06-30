# 2026-06-30 GGUF Test Organization Split

## Summary

The GGUF integration tests are now split by responsibility:

- `gguf_reader.rs` covers parser, metadata, tensor range, and tensor-shape
  guardrails.
- `gguf_config.rs` covers model config derivation and config-level metadata
  validation.
- `support/gguf.rs` owns shared GGUF byte-building fixtures.

This keeps the parser test target from growing into a mixed-purpose test file
while preserving the same behavioral coverage.

## Verification

Targeted checks after the split:

```text
cargo test -p ferrite-model --test gguf_reader -- --nocapture
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test -p ferrite-model --test gguf_config -- --nocapture
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
