# F32 matvec vector finite guardrail

## Context

F32 matrix construction now rejects non-finite stored values, but runtime matvec
inputs can still contain NaN or infinity values. The F32 matvec path dispatches
to scalar, NEON, or AVX2 implementations, so invalid vectors should be rejected
before backend-specific dot products run.

## Change

`f32_mul_vec` now rejects non-finite vector values after length validation and
before backend dispatch. This prevents invalid activations from propagating
through F32 scalar matvec output.

## Verification

- Red: `cargo test -p ferrite-inference --test scalar_reference matrix_vector_multiply_rejects_non_finite_vector_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test scalar_reference matrix_vector_multiply_rejects_non_finite_vector_values -- --nocapture`

