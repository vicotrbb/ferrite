# Q4_K matrix scale finite guardrail

## Context

Q4_K blocks store f16 `d` and `dmin` values before the packed per-subblock
scales and quantized values. Matrix construction validated byte length, but
invalid f16 bits could later decode to NaN or infinity during Q4_K row decoding
and matvec execution.

## Change

`Matrix::from_q4_k_row_major_bytes` now validates every Q4_K block has finite
`d` and `dmin` values before accepting the byte storage.

## Verification

- Red: `cargo test -p ferrite-inference --test matvec_kernel_check q4_k_matrix_rejects_non_finite_scale_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test matvec_kernel_check q4_k_matrix_rejects_non_finite_scale_values -- --nocapture`

