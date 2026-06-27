# 2026-06-27 Scalar Module Split Slice

## Scope

This slice starts reducing the size of the scalar inference module without
changing behavior.

## Implementation

- Extracted `Matrix` into `crates/ferrite-inference/src/scalar/matrix.rs`.
- Re-exported `Matrix` from `ferrite_inference::scalar` to preserve the public
  API used by existing tests and callers.
- Left scalar execution, loader, and math behavior unchanged.

## Evidence Plan

This is a behavior-preserving refactor. The existing scalar reference tests and
workspace tests are the verification gate.
