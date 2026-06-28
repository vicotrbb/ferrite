# 2026-06-27 Tier 1 SmolLM2 1.7B Q6_K Row Parallel

## Scope

This benchmark records the incremental effect of adding Rayon row parallelism
to the Q6_K SIMD matvec path after the Q4_K row-parallel slice.

This is still not a Tier 1 throughput pass. The 2-thread run remains below the
`>= 10 tok/s` target.

## Tree State

- Branch: `main`
- Commit before note: `6744f7f`
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
| Q4_K row parallel only, default Rayon pool | 558,353,433 | 1.791 | 5.73 s | 7.53 s | 1,473,675,264 bytes | 2,123,830,336 bytes |
| Q4_K + Q6_K row parallel, default Rayon pool | 317,917,433 | 3.145 | 3.81 s | 7.35 s | 1,475,821,568 bytes | 2,123,830,464 bytes |
| Q4_K row parallel only, `RAYON_NUM_THREADS=2` | 886,433,241 | 1.128 | 7.99 s | 6.72 s | 1,471,528,960 bytes | 2,123,912,448 bytes |
| Q4_K + Q6_K row parallel, `RAYON_NUM_THREADS=2` | 549,736,508 | 1.819 | 4.73 s | 6.46 s | 1,828,929,536 bytes | 2,123,551,872 bytes |

The Q4_K-only rows are from
`documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-row-parallel.md`.

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
        3.99 real         8.47 user         1.80 sys
          1537785856  maximum resident set size
          2123502656  peak memory footprint
````

Default Rayon-pool benchmark:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=1589587167
benchmark_avg_ns=317917433
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
        3.81 real         7.35 user         1.67 sys
          1475821568  maximum resident set size
          2123830464  peak memory footprint
```

Two-thread benchmark:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=2748682542
benchmark_avg_ns=549736508
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
        4.73 real         6.46 user         0.69 sys
          1828929536  maximum resident set size
          2123551872  peak memory footprint
```

## Interpretation

Adding Q6_K row parallelism on top of Q4_K row parallelism improves the local
default-pool cached-token average from about 0.558 seconds to about 0.318
seconds. The 2-thread average improves from about 0.886 seconds to about 0.550
seconds.

This is meaningful progress for the Tier 1 1.7B path, but the 2-thread result
is still about 1.82 tok/s, below the `>= 10 tok/s` target. Q8_0, Q5_0, F32
matvec, and higher-level decode scheduling remain likely optimization targets.
