# 2026-06-27 Tier 1 Q8_0 Row-Parallel Regression

## Scope

This note records a failed Tier 1 optimization experiment: applying the same
Rayon row-parallel SIMD pattern used for Q4_K and Q6_K to Q8_0 matvec.

The experiment was implemented in commit `3b12756` and reverted in commit
`1ae4275` after the real SmolLM2-1.7B benchmark regressed.

## Experiment

The Q8_0 SIMD backend was changed to:

- preserve the existing single-row SIMD backend marker;
- add a row-parallel backend marker for multi-row Q8_0 matrices; and
- schedule independent Q8_0 rows with Rayon.

A focused test was added first to require the new multi-row row-parallel backend
identity. The red check failed because the backend marker did not exist yet:

```text
no variant, associated function, or constant named `Aarch64NeonRowParallel`
found for enum `Q8_0MatVecBackend`
```

After implementation, focused Q8_0 tests passed and the real six-token Tier 1
reference check still matched the documented SmolLM2-1.7B profile.

## Regression Evidence

Benchmark command after Q8_0 row parallelism:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Output:

```text
benchmark_total_ns=3589029542
benchmark_avg_ns=717805908
        6.51 real         7.49 user         2.51 sys
          1473150976  maximum resident set size
          2123994240  peak memory footprint
```

Two-thread command:

```sh
RAYON_NUM_THREADS=2 /usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Output:

```text
benchmark_total_ns=5299960209
benchmark_avg_ns=1059992041
        7.68 real         6.78 user         1.52 sys
          1472430080  maximum resident set size
          2123912448  peak memory footprint
```

The prior Q4_K+Q6_K row-parallel benchmark note recorded:

- default pool: `benchmark_avg_ns=317917433`
- `RAYON_NUM_THREADS=2`: `benchmark_avg_ns=549736508`

The Q8_0 row-parallel experiment therefore regressed both local profiles.

## Revert Verification

After reverting `3b12756`, the release binary was rebuilt:

```sh
cargo build --release -p ferrite-cli
```

Default-pool benchmark after the revert:

```text
benchmark_total_ns=1354931208
benchmark_avg_ns=270986241
        3.68 real         7.34 user         1.82 sys
          1559658496  maximum resident set size
          2123666560  peak memory footprint
```

Two-thread benchmark after the revert:

```text
benchmark_total_ns=2600511167
benchmark_avg_ns=520102233
        4.49 real         6.37 user         0.67 sys
          1913651200  maximum resident set size
          2123387968  peak memory footprint
```

## Result

Naive Q8_0 row-level Rayon scheduling is not retained. It preserves correctness
but worsens the real Tier 1 benchmark on the local Apple M1 Pro host.

Future Q8_0 work should avoid copying the Q4_K/Q6_K row-parallel shape without
first isolating which Q8_0 tensors are hot and whether a higher threshold,
different scheduling strategy, or fused decode path is needed.
