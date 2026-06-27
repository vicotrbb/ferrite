# 2026-06-27 Tier 0 SmolLM2 Q6_K Direct Block Accumulation

## Scope

This benchmark records the Tier 0 SmolLM2 effect of replacing Q6_K matvec's
full decoded value-vector allocation with direct Q6_K block accumulation.

This is a scalar reference-path latency note. It is not an optimized production
quantized-matmul throughput claim.

## Tree State

- Branch: `main`
- Commit: `f58cfbb`
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
| 1 | 103,668,480 | 322,560 | 991,029,167 | 198,205,833 | 1.71 s | 1.39 s | 229,130,240 bytes | 223,954,688 bytes |
| 2 | 103,668,480 | 322,560 | 996,245,333 | 199,249,066 | 1.49 s | 1.40 s | 228,573,184 bytes | 224,216,704 bytes |
| 3 | 103,668,480 | 322,560 | 983,832,875 | 196,766,575 | 1.47 s | 1.39 s | 228,950,016 bytes | 224,102,144 bytes |

## Comparison

The prior Q4_K direct block accumulation note at commit `4dacb98` reported:

- `scalar_weight_bytes=103668480`
- Peak footprint about 224 MB
- Repeated benchmark average: 203-211 ms per accepted token

After direct Q6_K block accumulation:

- `scalar_weight_bytes=103668480`
- Peak footprint remains about 224 MB
- Repeated benchmark average: 197-199 ms per accepted token

## Interpretation

Direct Q6_K block accumulation removes the full decoded value-vector allocation
from Q6_K matvec and modestly improves the retained-weight scalar reference
path. With this slice, Q8_0, Q5_0, Q4_K, and Q6_K retained matrix matvecs all
avoid row-decode or full-decoded-matrix allocation in the inference matvec path.

The next Tier 0 performance work should be selected from measured evidence,
such as fusing quantized dot products, reducing repeated temporary allocations
around layer execution, or adding architecture-specific SIMD behind the scalar
reference boundary.
