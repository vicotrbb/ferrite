# 2026-06-27 Tier 0 SmolLM2 Q6_K Retained Direct Matvec

## Scope

This benchmark records the Tier 0 SmolLM2 effect of retaining Q6_K matrix bytes
and routing Q6_K matrix-vector multiplication through a decode-once scalar
matvec helper.

This is a scalar reference-path memory and latency note. It is not an optimized
production quantized-matmul throughput claim.

## Tree State

- Branch: `main`
- Retained-storage commit: `9ffc922`
- Direct-matvec commit: `c8e0754`
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

The single-token probe after direct Q6_K matvec produced the correct output and
memory numbers, but one sample had high wall time despite low CPU time:

| model_file_bytes | model_file_retained_bytes | scalar_weight_bytes | kv_cache_bytes | Real Time | User Time | Sys Time | Max RSS | Peak Footprint |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 105,454,432 | 0 | 103,668,480 | 92,160 | 14.51 s | 0.62 s | 0.05 s | 250,085,376 bytes | 224,216,768 bytes |

Because the built-in benchmark loop showed consistent CPU and wall-clock
results, use the repeated-token samples below for steady-state comparison.

## Repeated-Token Results

| Run | scalar_weight_bytes | kv_cache_bytes | benchmark_total_ns | benchmark_avg_ns | Real Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 103,668,480 | 322,560 | 1,530,426,042 | 306,085,208 | 2.22 s | 250,462,208 bytes | 223,971,136 bytes |
| 2 | 103,668,480 | 322,560 | 1,558,584,042 | 311,716,808 | 2.32 s | 250,576,896 bytes | 224,069,312 bytes |
| 3 | 103,668,480 | 322,560 | 1,577,101,333 | 315,420,266 | 2.34 s | 254,672,896 bytes | 224,233,088 bytes |

## Comparison

The prior Q4_K direct-matvec note at commit `0bf0f1d` reported:

- `scalar_weight_bytes=143053056`
- Peak footprint about 263 MB
- Repeated benchmark average: 266-275 ms per accepted token

After retaining Q6_K matrices and adding direct Q6_K scalar matvec:

- `scalar_weight_bytes=103668480`
- Peak footprint is about 224 MB
- Repeated benchmark average: 306-315 ms per accepted token

The scalar weight accounting dropped by 39,384,576 bytes relative to the Q4_K
direct-matvec baseline. The cost is roughly 31-49 ms per accepted token in this
small benchmark loop.

## Interpretation

Ferrite now retains Q8_0, Q5_0, Q4_K, and Q6_K matrix storage inside the custom
scalar model while preserving deterministic Tier 0 output for the SmolLM2
probe. The memory profile is now close to the compressed model size plus scalar
runtime overhead, with the raw GGUF bytes dropped after load.

The next performance work should avoid allocating a full decoded value vector
inside Q4_K and Q6_K matvec helpers and accumulate directly while decoding each
block.
