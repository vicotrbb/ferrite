# 2026-06-27 Shared Fixtures Slice

## Scope

This slice extracts generated GGUF test fixtures into a small dedicated crate so
future runtime and CLI tests can reuse them without duplicating large byte
builders.

## Implementation

- Added `crates/ferrite-fixtures`.
- Moved the scalar Llama-family F32, F16, and BF16 GGUF fixture generators out
  of `crates/ferrite-inference/tests/scalar_reference.rs`.
- Added `ferrite-fixtures` as a dev dependency for `ferrite-inference`.

## Boundaries

The fixture crate is test infrastructure. It does not add runtime behavior or
claim real Tier 0 model coverage.

## Evidence Plan

This is a behavior-preserving extraction. The existing workspace tests remain
the verification gate.
