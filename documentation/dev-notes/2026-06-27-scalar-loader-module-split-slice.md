# 2026-06-27 Scalar Loader Module Split Slice

## Scope

This slice extracts GGUF-to-scalar weight loading out of `scalar.rs`.

## Implementation

- Added `crates/ferrite-inference/src/scalar/loader.rs`.
- Kept `ScalarLlamaModel::from_gguf_f32` and
  `ScalarLlamaModel::from_gguf_unquantized` as the public loading boundary.
- Moved tensor lookup, shape checks, unquantized F32/F16/BF16 decoding, and
  GGUF config conversion into the loader module.

## Evidence Plan

This is a behavior-preserving refactor. Existing GGUF loader tests for F32, F16,
and BF16 fixtures plus the full workspace test suite remain the verification
gate.
