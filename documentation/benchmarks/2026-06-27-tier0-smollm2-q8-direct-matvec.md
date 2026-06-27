# 2026-06-27 Tier 0 SmolLM2 Q8_0 Direct Matvec

## Scope

This benchmark records the Tier 0 SmolLM2 effect of routing retained Q8_0
matrices through a direct scalar matrix-vector multiply helper.

This is a scalar reference-path latency note. It is not an optimized production
quantized-matmul throughput claim.

## Tree State

- Branch: `main`
- Commit: `3dec57e`
- Working tree before benchmark note: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 16 GB
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/SmolLM2-135M-Instruct-GGUF`
- File: `SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Hugging Face repo commit observed during download: `09816acd5d99df7be770d85ea30822623dab342c`
- Local path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 105,454,432 bytes
- Architecture: Llama-family GGUF
- Quantization: Q4_K_M GGUF mixture containing F32, Q8_0, Q5_0, Q4_K, and Q6_K tensors

## Commands

Build:

```sh
cargo build --release -p ferrite-cli
```

Repeated-token timed probe:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Prompt and output:

- Prompt text: `hello world`
- Prompt token IDs: `[28120, 905]`
- Initial output token ID: `30`
- Initial output token text: `.`
- Benchmark token IDs: `[198, 198, 57, 5248, 597]`
- Cached tokens after benchmark loop: 7
- Thread count: single-threaded Ferrite scalar path

## Repeated-Token Results

| Run | scalar_weight_bytes | kv_cache_bytes | benchmark_total_ns | benchmark_avg_ns | Real Time | User Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 103,668,480 | 322,560 | 1,480,876,417 | 296,175,283 | 3.70 s | 2.02 s | 254,918,656 bytes | 224,151,360 bytes |
| 2 | 103,668,480 | 322,560 | 1,470,467,292 | 294,093,458 | 3.67 s | 2.02 s | 250,822,656 bytes | 224,429,952 bytes |
| 3 | 103,668,480 | 322,560 | 1,484,225,250 | 296,845,050 | 3.69 s | 2.02 s | 254,246,912 bytes | 224,135,104 bytes |

The benchmark runs were launched in parallel, so process real time includes
machine-level contention. The in-process `benchmark_avg_ns` values are the
primary comparison signal for this note.

## Comparison

The prior Q6_K retained direct-matvec note at commit `c8e0754` reported:

- `scalar_weight_bytes=103668480`
- Peak footprint about 224 MB
- Repeated benchmark average: 306-315 ms per accepted token

After direct Q8_0 scalar matvec:

- `scalar_weight_bytes=103668480`
- Peak footprint remains about 224 MB
- Repeated benchmark average: 294-297 ms per accepted token

## Interpretation

Direct Q8_0 scalar matvec removes another row-decode allocation path while
preserving the retained matrix memory profile and deterministic Tier 0 output.
The measured effect is modest but aligned with the scalar reference path's next
optimization direction.

The next adjacent slice is direct Q5_0 scalar matvec, which is the remaining
retained quantized matrix type still using row decode plus scalar dot during
`Matrix::mul_vec`.
