# RMS norm epsilon boundary guardrail

## Context

The scalar model configuration path rejects invalid RMS norm epsilon values before
constructing a `ScalarLlamaModel`. The lower-level `rms_norm` helper is also a
public scalar API, so direct callers can bypass model configuration validation.

## Change

`rms_norm` now rejects epsilon values that are negative, NaN, or infinite before
computing the scale term. This keeps direct scalar math calls aligned with the
model configuration invariant and prevents invalid metadata or caller input from
propagating NaN values through normalization.

## Verification

- Red: `cargo test -p ferrite-inference --test scalar_reference rms_norm_rejects_invalid_epsilon -- --nocapture`
- Green: `cargo test -p ferrite-inference --test scalar_reference rms_norm_rejects_invalid_epsilon -- --nocapture`

