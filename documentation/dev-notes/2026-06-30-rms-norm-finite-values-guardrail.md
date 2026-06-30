# RMS Norm Finite Values Guardrail

## Context

`rms_norm` already rejected empty inputs and invalid epsilon values, but it did
not reject non-finite input or weight values. That let NaN or infinity enter the
normalization arithmetic and produce non-finite hidden states.

## Change

`rms_norm` now rejects NaN, positive infinity, and negative infinity in both the
input vector and the weight vector before computing the mean square.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference scalar::math::tests::rms_norm_rejects_non_finite_values -- --nocapture
```
