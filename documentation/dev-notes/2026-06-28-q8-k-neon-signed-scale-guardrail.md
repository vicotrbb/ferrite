# Q8_K NEON Signed-Scale Guardrail

Date: 2026-06-28

## Scope

This slice adds target-specific guardrail tests for the approved Path B
Q4_K/Q6_K x Q8_K aarch64 NEON helpers.

Previous reference tests proved the signed Q8_K activation-scale identity for
the scalar Q4_K and Q6_K Q8_K block-dot helpers. This slice extends that edge
coverage to the NEON helper boundary.

## Change

Added tests for both positive-dominant and negative-dominant activation blocks:

- `neon_q4_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales`
- `neon_q6_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales`

The tests compare the aarch64 NEON block-dot helpers against the scalar Q8_K
adapters after `BlockQ8K::quantize` selects either signed scale polarity.

## Verification

```sh
cargo test -p ferrite-inference neon_q4_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales -- --nocapture
cargo test -p ferrite-inference neon_q6_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales -- --nocapture
```

Results:

```text
neon_q4_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales ... ok
neon_q6_k_q8_k_block_dot_matches_scalar_for_signed_q8_k_scales ... ok
```

## Interpretation

This does not promote Q8_K activation matvec dispatch. It tightens confidence
that the existing experimental aarch64 helpers preserve the scalar Q8_K
contract for both activation-scale polarities.
