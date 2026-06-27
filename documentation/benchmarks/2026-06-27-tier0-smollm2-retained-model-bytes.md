# 2026-06-27 Tier 0 SmolLM2 Retained Model Bytes

## Scope

This benchmark records Ferrite's Tier 0 CLI memory accounting after dropping
the raw GGUF byte buffer once scalar loading completes.

This is retained-byte accounting. It is not a claim that process peak RSS drops
by the full GGUF file size, because peak RSS can include transient load-time
overlap.

## Tree State

- Branch: `main`
- Commit: `dc8398f`
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

| Run | model_file_bytes | model_file_retained_bytes | scalar_weight_bytes | kv_cache_bytes | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 105,454,432 | 0 | 538,060,032 | 322,560 | 658,407,424 bytes | 657,230,400 bytes |
| 2 | 105,454,432 | 0 | 538,060,032 | 322,560 | 662,437,888 bytes | 656,837,120 bytes |
| 3 | 105,454,432 | 0 | 538,060,032 | 322,560 | 662,044,672 bytes | 656,918,976 bytes |

Timing stayed in the same scalar-session range:

| Run | benchmark_total_ns | benchmark_avg_ns |
| --- | ---: | ---: |
| 1 | 538,901,250 | 107,780,250 |
| 2 | 570,054,750 | 114,010,950 |
| 3 | 567,307,042 | 113,461,408 |

## Interpretation

Ferrite now reports that the 105.5 MB raw GGUF file buffer is not retained
after scalar loading. The scalar weights still account for 538.1 MB, and the
live K/V cache for this seven-token run remains 322.6 KB.

The process-level peak footprint remains around the prior tied-output-memory
range because `/usr/bin/time -l` reports peak process memory, including
load-time overlap and allocator behavior. This note proves the CLI's retained
raw model buffer is removed; proving lower steady-state resident memory will
need a runtime sampler or a dedicated in-process memory probe.
