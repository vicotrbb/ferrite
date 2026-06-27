# 2026-06-27 Tier 0 SmolLM2 Tied Output Memory

## Scope

This benchmark records the Tier 0 SmolLM2 memory impact of avoiding the tied
output-weight clone in Ferrite's scalar loader.

This is a memory optimization note. It does not claim a throughput improvement.

## Tree State

- Branch: `main`
- Commit: `5bfad89`
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

## Command

Build:

```sh
cargo build --release -p ferrite-cli
```

Timed probe:

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

## Results

| Run | model_file_bytes | scalar_weight_bytes | kv_cache_bytes | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 105,454,432 | 538,060,032 | 322,560 | 661,831,680 bytes | 662,194,880 bytes |
| 2 | 105,454,432 | 538,060,032 | 322,560 | 662,863,872 bytes | 662,538,688 bytes |
| 3 | 105,454,432 | 538,060,032 | 322,560 | 662,421,504 bytes | 662,145,664 bytes |

Timing remained in the same scalar-session range:

| Run | benchmark_total_ns | benchmark_avg_ns |
| --- | ---: | ---: |
| 1 | 537,553,208 | 107,510,641 |
| 2 | 548,392,584 | 109,678,516 |
| 3 | 545,635,334 | 109,127,066 |

## Comparison

The prior memory accounting note at commit `973658f` reported:

- `scalar_weight_bytes=651306240`
- Peak footprint about 775 MB

After avoiding the tied output clone:

- `scalar_weight_bytes=538060032`
- Peak footprint about 662 MB

The scalar weight accounting dropped by 113,246,208 bytes, matching the removed
duplicate embedding-sized F32 matrix. Process peak footprint dropped by roughly
the same amount on this local run.

## Interpretation

SmolLM2-135M-Instruct Q4_K_M uses tied output embeddings in this GGUF artifact,
so materializing an owned output clone was pure scalar-loader overhead. Ferrite
now keeps one F32 embedding matrix and borrows it for logits when output weights
are tied.

The largest remaining Tier 0 memory cost is still dequantized F32
materialization of quantized tensors. The next memory-focused optimization
should retain quantized tensor storage for selected matrix multiplies or add
mmap-backed tensor access before moving to larger tiers.
