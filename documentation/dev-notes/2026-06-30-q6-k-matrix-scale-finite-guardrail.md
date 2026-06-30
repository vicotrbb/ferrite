# Q6_K Matrix Scale Finite Guardrail

## Context

`Matrix::from_q6_k_row_major_bytes` already verified shape-derived storage length, but it did not reject non-finite f16 super-block scale values. That allowed invalid bytes to enter a matrix and later produce non-finite matvec results through scalar or accelerated paths.

## Change

The Q6_K matrix constructor now scans each encoded block scale before storing the bytes and rejects NaN, positive infinity, and negative infinity with `Q6_K matrix scale values must be finite`.

The regression test covers all three non-finite f16 encodings used by the other quantized scale guardrails.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-inference --test matvec_kernel_check q6_k_matrix_rejects_non_finite_scale_values -- --nocapture
```
