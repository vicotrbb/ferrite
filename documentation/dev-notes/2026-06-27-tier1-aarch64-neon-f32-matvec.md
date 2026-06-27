# 2026-06-27 Tier 1 AArch64 NEON F32 Matvec

## Slice

This is Ferrite's first scoped SIMD kernel slice under ADR 0006. The workspace
unsafe-code lint changed from `forbid` to `deny` so an explicitly allowed,
module-local unsafe boundary can exist without making unsafe code broadly
acceptable.

The slice adds `crates/ferrite-inference/src/scalar/matvec.rs` with:

- a safe F32 matvec dispatcher;
- a scalar fallback;
- an aarch64 NEON implementation for F32 dot products;
- runtime NEON feature detection before dispatch;
- local safety comments for the unsafe intrinsic call; and
- an internal backend-selection test on aarch64.

The existing `Matrix::mul_vec` F32 path now uses this dispatcher. The public
`Matrix::mul_vec_checked_against_reference` gate compares the active F32 path
against the decoded scalar reference output, so on this host it checks the NEON
path against the scalar oracle.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference f32_matvec_uses_neon_backend_on_aarch64 -- --nocapture
```

The test failed on `aarch64-apple-darwin` because the dispatcher still reported
the scalar backend.

Passing checks after implementation:

```text
cargo test -p ferrite-inference f32_matvec_uses_neon_backend_on_aarch64 -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
```

The targeted aarch64 backend test passed, and the matvec reference-check target
passed all 3 tests.

## Remaining Work

This is an F32 NEON matvec slice only. It does not implement AVX2, AVX-512,
quantized SIMD kernels, threading, cache blocking, or Tier 1 throughput
evidence. The next SIMD slices should add x86_64 AVX2 and quantized-kernel
dispatch behind the same reference-comparison gate.
