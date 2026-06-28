# 2026-06-27 Tier 1 Q5_0 Row-Parallel Regression

## Scope

This note records a failed Tier 1 optimization experiment: applying Rayon
row-parallel SIMD scheduling to Q5_0 matvec.

The experiment was implemented in commit `f318e3b` and reverted in commit
`a5d9382` after the real SmolLM2-1.7B benchmark regressed.

## Experiment

The Q5_0 SIMD backend was changed to:

- preserve the existing single-row SIMD backend marker;
- add a row-parallel backend marker for multi-row Q5_0 matrices; and
- schedule independent Q5_0 rows with Rayon.

A focused test was added first to require the new multi-row row-parallel backend
identity. The red check failed because the backend marker did not exist yet:

```text
no variant, associated function, or constant named `Aarch64NeonRowParallel`
found for enum `Q5_0MatVecBackend`
```

After implementation, focused Q5_0 tests passed and the real six-token Tier 1
reference check still matched the documented SmolLM2-1.7B profile.

## Regression Evidence

Benchmark command after Q5_0 row parallelism:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Output:

```text
benchmark_total_ns=2098529500
benchmark_avg_ns=419705900
        4.48 real         7.46 user         1.91 sys
          1616691200  maximum resident set size
          2123895872  peak memory footprint
```

Two-thread command:

```sh
RAYON_NUM_THREADS=2 /usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Output:

```text
benchmark_total_ns=4253175333
benchmark_avg_ns=850635066
        7.65 real         6.73 user         1.38 sys
          1472708608  maximum resident set size
          2123551872  peak memory footprint
```

The retained Q4_K+Q6_K path had recently measured:

- default pool: `benchmark_avg_ns=270986241`
- `RAYON_NUM_THREADS=2`: `benchmark_avg_ns=520102233`

The Q5_0 row-parallel experiment therefore regressed both local profiles.

## Revert Verification

After reverting `f318e3b`, the release binary was rebuilt:

```sh
cargo build --release -p ferrite-cli
```

Default-pool benchmark after the revert:

```text
benchmark_total_ns=1305817417
benchmark_avg_ns=261163483
       16.18 real         7.20 user         1.92 sys
          1473921024  maximum resident set size
          2123650240  peak memory footprint
```

Two-thread benchmark after the revert:

```text
benchmark_total_ns=2883435125
benchmark_avg_ns=576687025
        4.52 real         6.40 user         0.71 sys
          2028486656  maximum resident set size
          2123568320  peak memory footprint
```

## Result

Naive Q5_0 row-level Rayon scheduling is not retained. It preserves correctness
but worsens the real Tier 1 benchmark on the local Apple M1 Pro host.

Future Q5_0 work should isolate which Q5_0 tensors are hot and test a threshold
or fused decode strategy before introducing row-level Rayon scheduling.
