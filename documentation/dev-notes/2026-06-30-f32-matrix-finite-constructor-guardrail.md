# F32 matrix finite constructor guardrail

## Context

`Matrix::from_row_major` is the scalar F32 weight entry point used by tests,
fixtures, and GGUF tensor loading after F32 conversion. It already validates the
row-major shape, but non-finite values could still be stored and later propagate
through matvec, attention, and token selection.

## Change

The F32 row-major matrix constructor now rejects NaN and infinity values after
shape validation and before storing matrix data.

## Verification

- Red: `cargo test -p ferrite-inference --test scalar_reference matrix_from_row_major_rejects_non_finite_values -- --nocapture`
- Green: `cargo test -p ferrite-inference --test scalar_reference matrix_from_row_major_rejects_non_finite_values -- --nocapture`

