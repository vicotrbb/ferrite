# 2026-06-27 Tier 1 AArch64 NEON Q5_0 Matvec

## Slice

Tier 1 still needed additional quantized SIMD coverage beyond Q8_0. This slice
adds an aarch64 NEON Q5_0 matvec path while preserving scalar fallback and row
decode behavior.

The implementation:

- moves Q5_0-specific matvec and row-decode logic into
  `crates/ferrite-inference/src/scalar/q5_0.rs`;
- keeps a scalar Q5_0 fallback;
- dispatches to NEON only after runtime feature detection;
- keeps local safety comments around the unsafe intrinsic call;
- leaves the generated GGUF Q5_0 fixture path covered by scalar reference
  integration tests; and
- keeps `quantized.rs` smaller by removing Q5_0 row-block internals from it.

## Validation

Test-first failure before implementation:

```text
cargo test -p ferrite-inference q5_0_matvec_uses_neon_backend_on_aarch64 -- --nocapture
```

The test failed because the `q5_0` module and backend-reporting API did not
exist.

Passing checks after implementation:

```text
cargo test -p ferrite-inference q5_0_matvec_uses_neon_backend_on_aarch64 -- --nocapture
cargo test -p ferrite-inference --test scalar_reference
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The targeted aarch64 Q5_0 backend test passed, the scalar reference target
passed all 16 tests, the matvec reference-check target passed all 3 tests, and
the full workspace test suite passed.

## Remaining Work

This proves Q5_0 NEON dispatch on the local aarch64 host. Tier 1 still needs
AVX2 evidence, Q4_K and Q6_K SIMD paths, real 0.5B-1.7B model output, and
throughput benchmarks before the tier can be considered complete.
