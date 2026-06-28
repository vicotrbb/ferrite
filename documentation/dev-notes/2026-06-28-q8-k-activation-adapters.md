# Q8_K Activation Adapters

Date: 2026-06-28

## Scope

This slice starts ADR 0007's approved activation-side `q8_K` path without yet
changing public Q4_K/Q6_K dispatch.

It adds:

- internal `BlockQ8K` activation blocks;
- activation block-vector quantization;
- scalar Q4_K x Q8_K block and row matvec adapters;
- scalar Q6_K x Q8_K block and row matvec adapters.

The Q4_K and Q6_K public dispatchers are intentionally unchanged in this slice.
The existing aarch64 NEON and x86_64 AVX2 backend-order tests still describe the
current public routing. Default dispatch should not be reordered until the
target-specific Q8_K helper path and model/reference gates are both proven.

## Red-Green Evidence

`BlockQ8K` started with failing tests for the missing activation block API:

```text
error[E0432]: unresolved imports `super::BlockQ8K`, `super::Q8_K_BLOCK_VALUES`, `super::Q8_K_GROUP_SIZE`, `super::Q8_K_GROUPS`
```

The non-finite activation guard was added after a failing test showed the
initial implementation accepted `f32::INFINITY`:

```text
Error: InferenceError { message: "non-finite activation must fail" }
```

The Q4_K adapter started with a failing test for the missing block-dot helper:

```text
error[E0432]: unresolved import `super::q4_k_q8_k_block_dot`
```

The Q6_K adapter started with a failing test for the missing block-dot helper:

```text
error[E0432]: unresolved import `super::q6_k_q8_k_block_dot`
```

The row adapters were added after failing tests for missing
`q4_k_q8_k_mul_vec` and `q6_k_q8_k_mul_vec` helpers.

## Verification

Focused checks passed before the relevant commits:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-inference q8_k -- --nocapture
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

The final row-adapter slice passed:

```text
cargo test -p ferrite-inference q4_k_q8_k -- --nocapture
2 passed; 0 failed

cargo test -p ferrite-inference q6_k_q8_k -- --nocapture
2 passed; 0 failed

cargo test -p ferrite-inference q8_k -- --nocapture
8 passed; 0 failed
```

## Current Limitations

- Q4_K/Q6_K public `mul_vec` dispatch still uses the pre-existing paths.
- The scalar Q8_K adapters are correctness contracts, not throughput wins.
- The temporary dead-code allowances on `q4_k_q8_k`, `q6_k_q8_k`, and `q8_k`
  should be removed when public or target-specific dispatch starts using these
  modules.
- No model-output or throughput claim is made by this slice.
