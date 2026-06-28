# Q8_K NEON Row Adapter Slice

Date: 2026-06-28

## Scope

This slice extends the internal aarch64 Q4_K x Q8_K and Q6_K x Q8_K helper path
from block-dot helpers to row-level adapters.

It adds:

- `neon_q4_k_q8_k_mul_vec`;
- `neon_q6_k_q8_k_mul_vec`;
- row-level tests comparing both NEON adapters against the scalar Q8_K
  adapters;
- scalar Q8_K adapter validation that rejects zero-column matrices before they
  can reach `chunks_exact(0)`.

This still does not change public Q4_K/Q6_K dispatch. The helpers remain
internal until model parity and benchmark gates justify a deliberate route
change.

## Red-Green Evidence

The row-adapter tests started red because the row helpers did not exist:

```text
error[E0432]: unresolved import `super::neon_q4_k_q8_k_mul_vec`
error[E0432]: unresolved import `super::neon_q6_k_q8_k_mul_vec`
```

While adding row validation, the scalar adapter zero-column regression tests
also exposed a panic:

```text
chunk size must be non-zero
```

Both cases were fixed with explicit adapters and validation.

## Verification

Focused checks passed after the row-adapter and validation commits:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-inference q8_k_neon -- --nocapture
cargo test -p ferrite-inference q8_k_mul_vec_rejects_zero_columns -- --nocapture
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

The focused test output included:

```text
neon_q4_k_q8_k_mul_vec_matches_scalar_q8_k_adapter ... ok
neon_q6_k_q8_k_mul_vec_matches_scalar_q8_k_adapter ... ok
q4_k_q8_k_mul_vec_rejects_zero_columns ... ok
q6_k_q8_k_mul_vec_rejects_zero_columns ... ok
```

## Current Limitations

- The row adapters are not wired into public dispatch.
- No model-output parity claim is made.
- No throughput claim is made.
- The Q6_K helper still uses scalar unpacking into temporary lanes before NEON
  dot accumulation; this is correctness-first, not final performance shape.
