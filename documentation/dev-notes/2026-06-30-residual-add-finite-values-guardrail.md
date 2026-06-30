# Residual Add Finite Values Guardrail

## Context

`add_assign` updates scalar hidden states with attention and FFN residuals, but
it only checked vector length compatibility. If a non-finite value reached the
residual path, it could silently propagate through later scalar reference
layers.

## Change

`add_assign` now rejects non-finite values in both the left hidden-state buffer
and the right residual vector before applying the in-place addition.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::math::tests::add_assign_rejects_non_finite_values -- --nocapture
```
