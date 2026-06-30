# 2026-06-30 GGUF Tensor Rank Guardrail

## Summary

Ferrite now rejects GGUF tensor metadata with more than four dimensions before
allocating the tensor shape vector.

GGML tensor metadata is four-dimensional in the model families Ferrite supports
today. Accepting arbitrary ranks would let malformed files drive unnecessary
allocation and defer rejection to later loader assumptions.

## Changes

- Added `MAX_TENSOR_DIMENSIONS` to the GGUF reader.
- Rejected tensor metadata whose dimension count exceeds that limit.
- Added a parser regression test for a five-dimensional tensor shape.

## Verification

Red test before implementation:

```text
cargo test -p ferrite-model --test gguf_reader rejects_tensors_with_too_many_dimensions -- --nocapture
test rejects_tensors_with_too_many_dimensions ... FAILED
Error: Custom { kind: Other, error: "over-rank tensor shape should be rejected" }
```

Focused green test after implementation:

```text
cargo test -p ferrite-model --test gguf_reader rejects_tensors_with_too_many_dimensions -- --nocapture
test rejects_tensors_with_too_many_dimensions ... ok
```
