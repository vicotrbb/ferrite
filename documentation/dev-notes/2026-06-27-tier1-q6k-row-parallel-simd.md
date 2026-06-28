# 2026-06-27 Tier 1 Q6_K Row-Parallel SIMD

## Scope

This slice adds row-level parallelism to the existing Q6_K SIMD matvec paths,
matching the Q4_K row-parallel structure.

It keeps the per-row Q6_K decode and SIMD dot-product logic unchanged. Rayon is
used only to schedule independent rows for the aarch64 NEON and x86_64 AVX2
backends.

## Code Changes

- Replaced the sequential row loop inside aarch64 NEON Q6_K matvec with
  `par_chunks_exact(row_bytes)`.
- Applied the same row-parallel structure to the x86_64 AVX2 Q6_K matvec.
- Added a Q6_K SIMD test with distinct rows to verify row order and sums after
  parallel scheduling.

## Validation

Focused red/green check:

```sh
cargo test -p ferrite-inference scalar::quantized::tests::q6_k -- --nocapture
```

The new ordering test initially failed because the expected fixture sum reused
a 128-column accumulation value. The corrected full-row expected sum is
`-8066.0`, and the focused test set passed after that correction.

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

Q6_K SIMD rows now execute through Rayon on supported aarch64 and x86_64
targets while preserving the scalar-reference validation boundary.

The local SmolLM2-1.7B benchmark improved again after this slice, but the
2-thread result remains below the Tier 1 throughput target.
