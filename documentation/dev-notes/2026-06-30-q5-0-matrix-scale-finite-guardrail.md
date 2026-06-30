# Q5_0 matrix scale finite guardrail

## Context

Q5_0 matrices store one f16 scale per 32-value block. Matrix construction
validated byte shape, but invalid f16 scale bits could still be accepted and
later decode to NaN or infinity during row decoding and matvec execution.

## Change

`Matrix::from_q5_0_row_major_bytes` now validates all Q5_0 block scales are
finite before accepting the byte storage.

## Verification

- Red: `cargo test -p ferrite-inference --test matvec_kernel_check q5_matrix_rejects_non_finite_scale_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test matvec_kernel_check q5_matrix_rejects_non_finite_scale_values -- --nocapture`

