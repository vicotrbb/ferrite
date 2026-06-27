# 2026-06-27 Tier 0 SmolLM2 Q5_0 Direct Matvec

## Scope

This benchmark records the Tier 0 SmolLM2 effect of routing retained Q5_0
matrices through a direct scalar matrix-vector multiply helper.

This is a scalar reference-path latency note. It is not an optimized production
quantized-matmul throughput claim.

## Tree State

- Branch: `main`
- Commit: `702d6ac`
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
| 1 | 103,668,480 | 322,560 | 1,087,238,708 | 217,447,741 | 12.61 s | 1.49 s | 249,823,232 bytes | 224,200,448 bytes |
| 2 | 103,668,480 | 322,560 | 1,085,258,084 | 217,051,616 | 1.62 s | 1.49 s | 250,904,576 bytes | 224,511,808 bytes |
| 3 | 103,668,480 | 322,560 | 1,120,708,417 | 224,141,683 | 1.68 s | 1.50 s | 249,954,304 bytes | 224,413,504 bytes |

Run 1 had high wall time despite normal user CPU time, so the in-process
`benchmark_avg_ns` values are the primary comparison signal.

## Comparison

The prior Q8_0 direct-matvec note at commit `3dec57e` reported:

- `scalar_weight_bytes=103668480`
- Peak footprint about 224 MB
- Repeated benchmark average: 294-297 ms per accepted token

After direct Q5_0 scalar matvec:

- `scalar_weight_bytes=103668480`
- Peak footprint remains about 224 MB
- Repeated benchmark average: 217-224 ms per accepted token

## Interpretation

Direct Q5_0 scalar matvec removes the last row-decode allocation path among the
currently retained quantized matrix types. This brings the retained-weight
scalar path back near the earlier scalar session-cache baseline while keeping
the compressed matrix memory profile.

The next performance work should reduce allocation inside the Q4_K and Q6_K
helpers themselves, which still decode full value vectors before accumulation.
