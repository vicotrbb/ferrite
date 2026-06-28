# Q8_K Q6_K Argmax Option Semantics

Date: 2026-06-28

## Scope

This slice closes a semantic hole in the experimental Q8_K activation matvec
route.

`Matrix::argmax_mul_vec_with_options` had a direct Q6_K argmax path. That path
is correct for default token-id-only decoding, but it ignored
`ScalarExecutionOptions::with_q8_k_activation_matvec(true)`. As a result,
experimental token-id-only generation or benchmark steps could run Q8_K for
hidden-layer Q4_K/Q6_K matvecs while still using the default Q6_K output
projection argmax.

## Red-Green Evidence

The regression test first failed:

```text
test q6_k_argmax_honors_q8_k_execution_options ... FAILED
assertion `left == right` failed
  left: 1
 right: 0
```

The fixture uses two Q6_K rows and an activation vector where the default f32
activation dot selects row 1, while Q8_K activation quantization ties the rows
and therefore selects row 0. The failure proved that the options-aware argmax
path was still using the default Q6_K argmax.

## Fix

On aarch64, Q6_K `argmax_mul_vec_with_options` now checks the experimental
Q8_K activation option before taking the default direct Q6_K argmax. When the
option is enabled, it computes the option-aware Q6_K matvec and then applies
the normal argmax.

Default Q6_K argmax dispatch remains unchanged.

## Verification

```sh
cargo test -p ferrite-inference --test matvec_kernel_check q6_k_argmax_honors_q8_k_execution_options -- --nocapture
cargo test -p ferrite-inference
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```

All commands passed.
