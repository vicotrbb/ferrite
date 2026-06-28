# Q8_K NEON Helper Slice

Date: 2026-06-28

## Scope

This slice adds aarch64 NEON block-dot helpers for the internal Q4_K x Q8_K and
Q6_K x Q8_K arithmetic contracts introduced by ADR 0007.

It adds:

- `q4_k_q8_k_neon.rs`;
- `q6_k_q8_k_neon.rs`;
- aarch64-only tests comparing each NEON helper against the scalar Q8_K adapter.

It does not change public Q4_K/Q6_K dispatch. Existing Q4_K/Q6_K matvec paths
and model execution remain on the prior dispatch order.

## Red-Green Evidence

Q4_K x Q8_K NEON started with a failing test for the missing helper:

```text
error[E0432]: unresolved import `super::neon_q4_k_q8_k_block_dot`
```

Q6_K x Q8_K NEON started with a failing test for the missing helper:

```text
error[E0432]: unresolved import `super::neon_q6_k_q8_k_block_dot`
```

Both helpers were then implemented behind focused modules and compared against
the scalar Q8_K adapters.

## Verification

Focused checks passed after the helper slices:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
cargo test -p ferrite-inference q8_k -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

The focused test output included:

```text
neon_q4_k_q8_k_block_dot_matches_scalar_q8_k_dot ... ok
neon_q6_k_q8_k_block_dot_matches_scalar_q8_k_dot ... ok
```

## Current Limitations

- The helpers are internal and temporarily covered by module-level dead-code
  allowances until dispatch is deliberately wired.
- Public Q4_K/Q6_K matvec dispatch is unchanged.
- No model-output parity or throughput claim is made by this slice.
