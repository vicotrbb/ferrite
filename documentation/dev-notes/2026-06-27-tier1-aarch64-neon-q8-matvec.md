# 2026-06-27 Tier 1 AArch64 NEON Q8_0 Matvec

## Slice

Tier 1 still needed a quantized SIMD correctness step. This slice adds an
aarch64 NEON Q8_0 matvec path while keeping the existing scalar and decoded
reference paths available.

The implementation:

- moves Q8_0-specific matvec and row-decode logic into
  `crates/ferrite-inference/src/scalar/q8_0.rs`;
- moves shared F16 decoding into `crates/ferrite-inference/src/scalar/float.rs`;
- keeps a scalar Q8_0 fallback;
- dispatches to NEON only after runtime feature detection;
- keeps local safety comments around the unsafe intrinsic call; and
- leaves `Matrix::mul_vec_checked_against_reference` as the correctness gate
  comparing active Q8_0 output against decoded scalar reference output.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference q8_0_matvec_uses_neon_backend_on_aarch64 -- --nocapture
```

The test failed because `q8_0_mul_vec_with_backend` and `Q8_0MatVecBackend`
did not exist.

Passing checks after implementation:

```text
cargo test -p ferrite-inference q8_0_matvec_uses_neon_backend_on_aarch64 -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The targeted aarch64 Q8_0 backend test passed, the matvec reference-check target
passed all 3 tests, and the full workspace test suite passed.

## Remaining Work

This proves one quantized NEON path on the local aarch64 host. Tier 1 still
needs AVX2 evidence, Q4_K/Q5_0/Q6_K SIMD paths, real 0.5B-1.7B model output,
and throughput benchmarks before the tier can be considered complete.
