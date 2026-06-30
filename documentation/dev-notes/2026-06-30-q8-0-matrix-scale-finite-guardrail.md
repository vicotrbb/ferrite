# Q8_0 matrix scale finite guardrail

## Context

Q8_0 matrices store one f16 scale per 32-value block. Matrix construction
validated byte shape, but invalid f16 scale bits could be stored and later
decoded as NaN or infinity during matvec and row decoding.

## Change

`Matrix::from_q8_0_row_major_bytes` now validates all Q8_0 block scale values
are finite before accepting the byte storage.

## Verification

- Red: `cargo test -p ferrite-inference --test matvec_kernel_check q8_matrix_rejects_non_finite_scale_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test matvec_kernel_check q8_matrix_rejects_non_finite_scale_values -- --nocapture`

