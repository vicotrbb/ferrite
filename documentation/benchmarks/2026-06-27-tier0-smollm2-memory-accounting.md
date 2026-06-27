# 2026-06-27 Tier 0 SmolLM2 Memory Accounting

## Scope

This benchmark records Ferrite's first component memory accounting for the Tier
0 SmolLM2 Q4_K_M scalar path. It measures the CLI process after loading the
model, accepting the prompt `hello world`, and accepting five benchmark tokens
with a scalar session cache.

This is a memory accounting note, not a new throughput optimization claim.

## Tree State

- Branch: `main`
- Commit: `973658f`
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
- Repeated benchmark calls per process: 5
- Cached tokens after benchmark loop: 7
- Benchmark token IDs: `[198, 198, 57, 5248, 597]`
- Thread count: single-threaded Ferrite scalar path

## Results

| Run | model_file_bytes | scalar_weight_bytes | kv_cache_bytes | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 105,454,432 | 651,306,240 | 322,560 | 769,736,704 bytes | 775,703,424 bytes |
| 2 | 105,454,432 | 651,306,240 | 322,560 | 775,208,960 bytes | 775,342,656 bytes |
| 3 | 105,454,432 | 651,306,240 | 322,560 | 774,733,824 bytes | 775,965,312 bytes |

Timing stayed in the same scalar-session range as the previous session-cache
baseline:

| Run | benchmark_total_ns | benchmark_avg_ns |
| --- | ---: | ---: |
| 1 | 537,683,291 | 107,536,658 |
| 2 | 702,484,917 | 140,496,983 |
| 3 | 619,468,667 | 123,893,733 |

## Interpretation

The scalar path currently expands a 105.5 MB GGUF file into 651.3 MB of owned
F32 scalar weight storage. After the prompt plus five accepted benchmark
tokens, the live scalar K/V cache is only 322.6 KB. The process-level peak
footprint remains about 775 MB, so the remaining memory is attributable to a
mix of the loaded GGUF byte buffer, tokenizer and GGUF metadata, vector
capacity slack, allocator overhead, runtime buffers, stack/code pages, and
operating-system effects.

This confirms the current Tier 0 memory bottleneck is dequantized F32 weight
materialization, not the K/V cache. Future memory work should prioritize
retaining quantized tensors, mmap-backed tensor access, avoiding avoidable
weight clones, and then measuring tokenizer and metadata overhead separately.
