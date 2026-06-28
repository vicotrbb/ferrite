# 2026-06-27 Tier 1 Qwen2 Q4_K_M Throughput

## Scope

This benchmark records local decode throughput for the two Tier 1 Qwen2 Q4_K_M
models that now have deterministic reference-token proofs:

- Qwen2.5-0.5B-Instruct Q4_K_M
- Qwen2.5-1.5B-Instruct Q4_K_M

This is benchmark evidence only. It is not a Tier 1 throughput pass because all
timed runs remain below the `>= 10 tok/s` target.

## Tree State

- Branch: `main`
- Commit before note: `30607b6`
- Working tree before note: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- Physical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

Commands:

```sh
sysctl -n machdep.cpu.brand_string hw.ncpu hw.physicalcpu hw.logicalcpu
sysctl -n hw.memsize
uname -a
```

## Models

### Qwen2.5-0.5B-Instruct Q4_K_M

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 397,808,192 bytes
- Scalar weight bytes reported by Ferrite: 391,859,712 bytes

### Qwen2.5-1.5B-Instruct Q4_K_M

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- File: `qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local size reported by Ferrite: 1,117,320,736 bytes
- Scalar weight bytes reported by Ferrite: 1,111,370,240 bytes

## Protocol

- Prompt: `hello world`
- Benchmark runs: 5 repeated generated-token steps after the initial prompt
  next-token computation
- Thread variants:
  - default Rayon pool
  - `RAYON_NUM_THREADS=2`
- Memory evidence: `/usr/bin/time -l`

## Results

| Model | Thread setting | benchmark_avg_ns | Approx tok/s | Real time | User time | Max RSS | Peak footprint |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5-0.5B Q4_K_M | default Rayon pool | 272,478,316 | 3.67 | 3.66 s | 1.65 s | 827,686,912 bytes | 827,804,928 bytes |
| Qwen2.5-0.5B Q4_K_M | `RAYON_NUM_THREADS=2` | 250,713,558 | 3.99 | 4.12 s | 1.57 s | 643,104,768 bytes | 828,099,840 bytes |
| Qwen2.5-1.5B Q4_K_M | default Rayon pool | 308,578,816 | 3.24 | 5.37 s | 4.35 s | 2,089,861,120 bytes | 2,268,108,288 bytes |
| Qwen2.5-1.5B Q4_K_M | `RAYON_NUM_THREADS=2` | 417,830,566 | 2.39 | 6.07 s | 3.65 s | 2,082,488,320 bytes | 2,268,321,408 bytes |

## Evidence

Qwen2.5-0.5B default pool:

```text
benchmark_runs=5
benchmark_total_ns=1362391583
benchmark_avg_ns=272478316
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
        3.66 real         1.65 user         0.62 sys
           827686912  maximum resident set size
           827804928  peak memory footprint
```

Qwen2.5-0.5B with two Rayon threads:

```text
benchmark_runs=5
benchmark_total_ns=1253567792
benchmark_avg_ns=250713558
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
        4.12 real         1.57 user         0.51 sys
           643104768  maximum resident set size
           828099840  peak memory footprint
```

Qwen2.5-1.5B default pool:

```text
benchmark_runs=5
benchmark_total_ns=1542894083
benchmark_avg_ns=308578816
model_file_bytes=1117320736
scalar_weight_bytes=1111370240
kv_cache_bytes=401408
        5.37 real         4.35 user         2.35 sys
          2089861120  maximum resident set size
          2268108288  peak memory footprint
```

Qwen2.5-1.5B with two Rayon threads:

```text
benchmark_runs=5
benchmark_total_ns=2089152833
benchmark_avg_ns=417830566
model_file_bytes=1117320736
scalar_weight_bytes=1111370240
kv_cache_bytes=401408
        6.07 real         3.65 user         1.51 sys
          2082488320  maximum resident set size
          2268321408  peak memory footprint
```

## Interpretation

The current Qwen2 path is correct enough for the fixed deterministic reference
profiles, but it remains far below the Tier 1 throughput target on this local
Apple M1 Pro host.

The Qwen2.5-0.5B two-thread run was the fastest timed Qwen2 result in this
slice at about 3.99 tok/s. Qwen2.5-1.5B regressed when restricted to two Rayon
threads, which matches the broader project pattern that thread-count tuning
needs model-specific evidence.

Next performance work should profile these Qwen2 models before changing
scheduling. The result does not justify a throughput claim.
