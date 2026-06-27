# 2026-06-27 Tier 0 SmolLM2 Q4_K Direct Matvec

## Scope

This benchmark records the Tier 0 SmolLM2 latency effect of replacing
row-by-row full-matrix Q4_K decode with direct scalar Q4_K matrix-vector
multiplication.

This is a scalar reference-path improvement. It is not an optimized production
quantized-matmul throughput claim.

## Tree State

- Branch: `main`
- Commit: `0bf0f1d`
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

Single-token timed probe:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world'
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

## Single-Token Result

| model_file_bytes | model_file_retained_bytes | scalar_weight_bytes | kv_cache_bytes | Real Time | User Time | Sys Time | Max RSS | Peak Footprint |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 105,454,432 | 0 | 143,053,056 | 92,160 | 0.89 s | 0.56 s | 0.06 s | 281,837,568 bytes | 263,194,496 bytes |

## Repeated-Token Results

| Run | scalar_weight_bytes | kv_cache_bytes | benchmark_total_ns | benchmark_avg_ns | Real Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 143,053,056 | 322,560 | 1,361,137,334 | 272,227,466 | 2.02 s | 281,985,024 bytes | 263,358,272 bytes |
| 2 | 143,053,056 | 322,560 | 1,373,618,375 | 274,723,675 | 2.02 s | 290,308,096 bytes | 263,341,824 bytes |
| 3 | 143,053,056 | 322,560 | 1,329,550,667 | 265,910,133 | 1.97 s | 289,619,968 bytes | 262,866,560 bytes |

## Comparison

The prior Q4_K retained-matrix note at commit `6ffb12c` reported:

- `scalar_weight_bytes=143053056`
- Peak footprint about 263 MB
- Single-token prompt probe time: 14.26 s

After direct Q4_K scalar matvec:

- `scalar_weight_bytes=143053056`
- Peak footprint remains about 263 MB
- Single-token prompt probe time: 0.89 s
- Repeated benchmark average: 266-275 ms per accepted token

## Interpretation

The direct Q4_K scalar matvec removes the severe row-by-row full-matrix decode
regression while preserving the retained Q4_K memory footprint. It is still
slower than the pre-Q4_K retained path because the scalar path decodes
quantized blocks during every matrix-vector multiply.

The next performance slice should avoid allocating a full decoded value vector
inside `q4_k_mul_vec` and instead accumulate directly while decoding each
block. After that, Q6_K retained storage can be added with the same discipline.
