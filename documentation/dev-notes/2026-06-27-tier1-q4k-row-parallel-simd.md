# 2026-06-27 Tier 1 Q4_K Row-Parallel SIMD

## Scope

This slice adds row-level parallelism to the existing Q4_K SIMD matvec paths.

It keeps the existing scalar and SIMD numerical kernels intact: each matrix row
still computes the same per-row dot product, but independent rows are scheduled
with Rayon for the aarch64 NEON and x86_64 AVX2 Q4_K backends.

## Code Changes

- Added `rayon` to `ferrite-inference`.
- Replaced the sequential row loop inside aarch64 NEON Q4_K matvec with
  `par_chunks_exact(row_bytes)`.
- Applied the same row-parallel structure to the x86_64 AVX2 Q4_K matvec so the
  compile-checked x86 path stays structurally aligned.
- Added a Q4_K SIMD test that verifies multi-row output order and sums are
  preserved by the row-parallel path.

## Validation

Focused red/green check:

```sh
cargo test -p ferrite-inference scalar::quantized::tests::q4_k -- --nocapture
```

Final code gates:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

All commands passed.

Real Tier 1 model parity check after rebuilding release:

```sh
cargo build --release -p ferrite-cli
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788
```

Ferrite still matched the documented six-token reference profile:

```text
generated_token_ids=18,198,3725,198,198,788
generated_match=true
expected_token_id=18
match=true
```

## Result

Q4_K SIMD rows now execute through Rayon on supported aarch64 and x86_64
targets while preserving the existing scalar-reference validation boundary.

This materially improves the local SmolLM2-1.7B Q4_K_M path, but it is not a
full Tier 1 throughput pass. The remaining matvec formats, decode scheduling,
and 2-vCPU target still need additional work.
