# 2026-06-27 Tier 1 AArch64 NEON Q4_K Matvec

## Slice

Tier 1 still needed Q4_K SIMD coverage after the Q8_0, Q5_0, and Q6_K NEON
paths. This slice adds an aarch64 NEON Q4_K matvec path while preserving scalar
fallback behavior for shapes that do not align to whole Q4_K blocks per row.

The implementation:

- moves Q4_K-specific decode and matvec logic into
  `crates/ferrite-inference/src/scalar/q4_k.rs`;
- keeps a scalar Q4_K fallback for existing row-spanning fixture shapes;
- dispatches to NEON only after runtime feature detection and only when `cols`
  is a non-zero multiple of 256;
- keeps local safety comments around the unsafe intrinsic call;
- wires `Matrix::mul_vec` for Q4_K through the new dispatcher; and
- keeps `quantized.rs` smaller by removing Q4_K internals from it.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference q4_k_matvec_uses_neon_backend_on_aarch64 -- --nocapture
```

The test failed because the `q4_k` module and backend-reporting API did not
exist.

Passing checks after implementation:

```text
cargo test -p ferrite-inference q4_k_matvec_uses_neon_backend_on_aarch64 -- --nocapture
cargo test -p ferrite-inference --test scalar_reference
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The targeted aarch64 Q4_K backend test passed, the scalar reference target
passed all 16 tests, the matvec reference-check target passed all 3 tests, and
the full workspace test suite passed.

## Remaining Work

This proves Q4_K NEON dispatch on the local aarch64 host for rows whose column
count is a whole number of Q4_K blocks. Tier 1 still needs AVX2 evidence, real
0.5B-1.7B model output, and throughput benchmarks before the tier can be
considered complete.
