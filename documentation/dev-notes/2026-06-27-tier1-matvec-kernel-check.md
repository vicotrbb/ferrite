# 2026-06-27 Tier 1 Matvec Kernel Check

## Slice

Tier 1 requires optimized kernels to match Ferrite's scalar reference output
within a 0.1% relative-error tolerance before they can be trusted. The workspace
currently forbids unsafe code, so this slice does not add SIMD intrinsics.
Instead, it adds the correctness harness optimized matvec paths must pass.

This slice adds:

- `Matrix::mul_vec_checked_against_reference(vector, tolerance)`;
- `scalar/kernel_check.rs` for relative-error validation; and
- `crates/ferrite-inference/tests/matvec_kernel_check.rs` covering F32 and
  Q8_0 matvec checks against decoded scalar reference output.

The check returns the active matvec output after comparing it against the
decoded scalar reference path.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

The new tests failed because `Matrix::mul_vec_checked_against_reference` did
not exist.

Passing checks after implementation:

```text
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo test -p ferrite-inference --test scalar_reference
```

The new matvec kernel-check target passed all 3 tests, and the scalar reference
integration target passed all 16 tests.

## Remaining Work

This is a correctness gate for future optimized kernels, not an AVX2 or
throughput implementation. Durable SIMD work still needs an unsafe-boundary ADR
or lint-policy change, then optimized kernels must be run through this
comparison path before correctness or Tier 1 throughput claims are made.
