# Q8_K Row Shape Guardrail

Date: 2026-06-28

## Scope

This slice hardens ADR 0007 Path B after approving it as an experimental
parity-scoped path. The Q4_K/Q6_K x Q8_K adapters require each matrix row to be
a whole number of 256-value K-blocks because each row block is dotted against
one `BlockQ8K` activation block.

The public dispatchers already restrict Q8_K activation matvec dispatch to
whole-block rows. This slice makes the same invariant explicit inside the
scalar and aarch64 NEON Q8_K row adapters, so direct internal calls cannot
silently rely on lower-level activation quantization errors.

## Red Check

The new partial-row tests failed before the validation change:

```text
q4_k_q8_k_mul_vec_rejects_partial_block_columns ... FAILED
q6_k_q8_k_mul_vec_rejects_partial_block_columns ... FAILED
neon_q4_k_q8_k_mul_vec_rejects_partial_block_columns ... FAILED
neon_q6_k_q8_k_mul_vec_rejects_partial_block_columns ... FAILED
left: "Q8_K activation length 128 must be divisible by 256"
right: "Q4_K Q8_K columns 128 must be divisible by 256"
```

## Change

- `q4_k_q8_k_mul_vec` now rejects non-256-divisible columns before activation
  quantization.
- `q6_k_q8_k_mul_vec` now rejects non-256-divisible columns before activation
  quantization.
- The aarch64 NEON Q4_K/Q6_K x Q8_K row adapters enforce the same invariant and
  have matching tests.

## Verification

```sh
cargo test -p ferrite-inference partial_block_columns -- --nocapture
cargo test -p ferrite-inference q8_k -- --nocapture
```

Both commands passed after the fix.

## Boundary

This does not promote Path B to default dispatch. It only tightens the approved
experimental parity-scoped design by making a required row-shape invariant
explicit at every Q8_K row-adapter entry point.
