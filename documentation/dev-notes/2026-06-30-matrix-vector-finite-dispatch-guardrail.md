# Matrix vector finite dispatch guardrail

## Context

The F32 matvec kernel rejects non-finite vector inputs, but quantized matrix
dispatches such as Q8_0 validate shape and then route directly into scalar or
SIMD dot products. That left public `Matrix::mul_vec` calls with quantized
storage able to propagate NaN or infinity activations.

## Change

The public matrix matvec entry points now reject non-finite vector values before
storage-specific dispatch. The guard applies to `mul_vec_with_options`,
`argmax_mul_vec_with_options`, and the scalar-reference comparison path.

## Verification

- Red: `cargo test -p ferrite-inference --test matvec_kernel_check q8_matvec_rejects_non_finite_vector_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test matvec_kernel_check q8_matvec_rejects_non_finite_vector_values -- --nocapture`

