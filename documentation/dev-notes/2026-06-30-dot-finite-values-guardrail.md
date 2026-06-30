# Dot Finite Values Guardrail

## Context

`dot` checked vector length compatibility, but it did not reject NaN or infinity
before multiplying operands. This could let non-finite attention scores or
matrix products escape from scalar reference paths if an upstream invariant
regressed.

## Change

`dot` now rejects non-finite values in both operands before computing the
elementwise product sum.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::math::tests::dot_rejects_non_finite_values -- --nocapture
```
