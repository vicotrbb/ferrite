# 2026-06-27 Tier 1 SmolLM2 1.7B Q4_K Row Parallel

## Scope

This benchmark records the effect of adding Rayon row parallelism to the Q4_K
SIMD matvec path for the local SmolLM2-1.7B-Instruct Q4_K_M Tier 1 probe.

This is an optimization benchmark, not a Tier 1 throughput pass. The current
implementation still falls below the `>= 10 tok/s` 2-vCPU target.

## Tree State

- Branch: `main`
- Commit before note: `3310fd4`
- Working tree before note: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 16 GB
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/SmolLM2-1.7B-Instruct-GGUF`
- File: `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 1,055,609,824 bytes
- Scalar weight bytes reported by Ferrite: 1,053,827,072 bytes
- Quantization: Q4_K_M GGUF mixture

## Commands

Release build:

```sh
cargo build --release -p ferrite-cli
```

Parity check:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788
```

Default Rayon-pool benchmark:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Two-thread benchmark:

```sh
RAYON_NUM_THREADS=2 /usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

## Results

| Run | benchmark_avg_ns | Approx Tok/s | Real Time | User Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar baseline before Q4_K row parallel | 6,333,709,400 | 0.158 | 45.76 s | 42.98 s | 1,471,365,120 bytes | 2,123,404,416 bytes |
| Q4_K row parallel, default Rayon pool | 558,353,433 | 1.791 | 5.73 s | 7.53 s | 1,473,675,264 bytes | 2,123,830,336 bytes |
| Q4_K row parallel, `RAYON_NUM_THREADS=2` | 886,433,241 | 1.128 | 7.99 s | 6.72 s | 1,471,528,960 bytes | 2,123,912,448 bytes |

The baseline row is from
`documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-scalar-probe.md`.

## Full Output

Parity check:

````text
prompt_token_ids=28120,905
next_token_id=18
next_token="
generated_cached_tokens=8
generated_token_ids=18,198,3725,198,198,788
generated_text="
```

In
expected_generated_token_ids=18,198,3725,198,198,788
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=3145728
expected_token_id=18
match=true
        5.10 real         8.46 user         1.82 sys
          1653604352  maximum resident set size
          2123748544  peak memory footprint
````

Default Rayon-pool benchmark:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=2791767167
benchmark_avg_ns=558353433
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
        5.73 real         7.53 user         1.87 sys
          1473675264  maximum resident set size
          2123830336  peak memory footprint
```

Two-thread benchmark:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=4432166208
benchmark_avg_ns=886433241
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
        7.99 real         6.72 user         1.35 sys
          1471528960  maximum resident set size
          2123912448  peak memory footprint
```

## Interpretation

Q4_K row parallelism moves the local default-pool cached-token average from
about 6.33 seconds to about 0.558 seconds for this probe, while preserving the
documented six-token reference output.

The 2-thread run averaged about 0.886 seconds per cached accepted token, or
about 1.13 tok/s. That is a large improvement over the scalar baseline, but it
is still below the Tier 1 `>= 10 tok/s` target. Remaining work should focus on
the other hot formats and decode-level scheduling before making throughput
claims.
