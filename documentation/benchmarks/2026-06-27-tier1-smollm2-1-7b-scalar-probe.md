# 2026-06-27 Tier 1 SmolLM2 1.7B Scalar Probe

## Scope

This benchmark records Ferrite's first scalar reference-path timing and memory
probe for the Tier 1 SmolLM2-1.7B-Instruct Q4_K_M GGUF artifact.

This is not an optimized throughput claim. It measures the current
single-threaded scalar reference path on the local host.

## Tree State

- Branch: `main`
- Commit before note: `325c960`
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
- Hugging Face repo revision recorded by local cache:
  `1f03464768bfcc0319fc50da8ff5fb20b6417ba2`
- Quantization: Q4_K_M GGUF mixture

## Commands

Repeated-token timed probe:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Prompt and output:

- Prompt text: `hello world`
- Prompt token IDs: `[28120, 905]`
- Initial output token ID: `18`
- Initial output token text: `"`
- Benchmark token IDs: `[198, 3725, 198, 198, 788]`
- Cached tokens after benchmark loop: 7
- Thread count: single-threaded Ferrite scalar path

## Results

| scalar_weight_bytes | kv_cache_bytes | benchmark_total_ns | benchmark_avg_ns | Approx Tok/s | Real Time | User Time | Max RSS | Peak Footprint |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,053,827,072 | 2,752,512 | 31,668,547,000 | 6,333,709,400 | 0.158 | 45.76 s | 42.98 s | 1,471,365,120 bytes | 2,123,404,416 bytes |

Full output:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=198,3725,198,198,788
benchmark_total_ns=31668547000
benchmark_avg_ns=6333709400
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
       45.76 real        42.98 user         1.07 sys
          1471365120  maximum resident set size
          2123404416  peak memory footprint
```

## Interpretation

The current scalar path can load and run a real 1.7B Tier 1 GGUF model on the
local Apple M1 Pro host, but it is far below the Tier 1 throughput target. The
local scalar benchmark averaged about 6.33 seconds per repeated accepted token,
or about 0.158 tokens per second.

This benchmark should be treated as a baseline for SIMD and decode-path
optimization work, not as evidence that Tier 1 performance is complete.
