# Softmax finite boundary guardrail

## Context

Scalar attention normalizes score vectors with `softmax`. The helper already
rejected empty inputs, but non-finite scores could flow into the max and
exponential passes and produce invalid probability vectors.

## Change

`softmax` now rejects NaN and infinity values before computing the maximum score.
This makes invalid attention scores fail fast instead of silently propagating
NaN or infinity through attention probabilities.

## Verification

- Red: `cargo test -p ferrite-inference scalar::math::tests::softmax_rejects_non_finite_values -- --nocapture`
- Green: `cargo test -p ferrite-inference scalar::math::tests::softmax_rejects_non_finite_values -- --nocapture`

