# RoPE frequency boundary guardrail

## Context

Ferrite already validates scalar model configuration so invalid
`rope_freq_base` metadata is rejected before inference. The public `apply_rope`
helper also accepts a frequency base directly, which means callers can bypass
that model configuration validation.

## Change

The shared RoPE implementation now rejects non-finite frequency bases before
checking positivity or computing rotary angles. This prevents NaN and infinity
inputs from producing invalid trigonometric results at the public scalar helper
boundary.

## Verification

- Red: `cargo test -p ferrite-inference --test scalar_reference apply_rope_rejects_non_finite_frequency_base -- --nocapture`
- Green: `cargo test -p ferrite-inference --test scalar_reference apply_rope_rejects_non_finite_frequency_base -- --nocapture`

