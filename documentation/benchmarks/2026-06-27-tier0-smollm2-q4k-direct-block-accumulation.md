# 2026-06-27 Tier 0 SmolLM2 Q4_K Direct Block Accumulation

## Scope

This benchmark records the Tier 0 SmolLM2 effect of replacing Q4_K matvec's
full decoded value-vector allocation with direct Q4_K block accumulation.

This is a scalar reference-path latency note. It is not an optimized production
quantized-matmul throughput claim.

## Tree State

- Branch: `main`
- Commit: `4dacb98`
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
| 1 | 103,668,480 | 322,560 | 1,055,990,959 | 211,198,191 | 1.77 s | 1.41 s | 246,497,280 bytes | 224,085,696 bytes |
| 2 | 103,668,480 | 322,560 | 1,016,756,959 | 203,351,391 | 1.51 s | 1.40 s | 250,396,672 bytes | 224,036,544 bytes |
| 3 | 103,668,480 | 322,560 | 1,035,514,584 | 207,102,916 | 1.54 s | 1.42 s | 250,789,888 bytes | 223,921,920 bytes |

## Comparison

The prior Q5_0 direct-matvec note at commit `702d6ac` reported:

- `scalar_weight_bytes=103668480`
- Peak footprint about 224 MB
- Repeated benchmark average: 217-224 ms per accepted token

After direct Q4_K block accumulation:

- `scalar_weight_bytes=103668480`
- Peak footprint remains about 224 MB
- Repeated benchmark average: 203-211 ms per accepted token

## Interpretation

Direct Q4_K block accumulation removes the full decoded value-vector allocation
from Q4_K matvec and modestly improves the retained-weight scalar reference
path. The retained memory profile and deterministic Tier 0 output are
unchanged.

The adjacent remaining allocation-heavy helper is Q6_K matvec, which still
decodes a full value vector before accumulation.
