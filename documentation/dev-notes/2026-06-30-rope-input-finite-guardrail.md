# RoPE Input Finite Guardrail

## Context

The shared scalar RoPE helper already rejected invalid rotary widths and
non-finite frequency bases, but it trusted the input vector values. A non-finite
query or key activation could therefore pass through RoPE, including the
zero-rotation fast path.

## Change

`apply_rope_with_layout` now rejects non-finite input values before handling the
zero-rotation path or applying either adjacent-pair or split-half rotation.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::rope::tests::rope_rejects_non_finite_values -- --nocapture
```
