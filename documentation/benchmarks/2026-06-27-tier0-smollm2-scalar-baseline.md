# 2026-06-27 Tier 0 SmolLM2 Scalar Baseline

## Scope

This benchmark records Ferrite's first local Tier 0 scalar baseline for a real
SmolLM2 Q4_K_M GGUF model. It measures end-to-end CLI execution: model file
read, GGUF parsing, tensor dequantization into scalar matrices, prompt
tokenization, scalar forward pass, and output printing.

This is not a steady-state decode throughput benchmark.

## Tree State

- Branch: `main`
- Commit: `e56720a`
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
- Local size: 101 MB
- Architecture: Llama-family GGUF
- Quantization: Q4_K_M GGUF mixture containing F32, Q8_0, Q5_0, Q4_K, and Q6_K tensors

## Command

Build:

```sh
cargo build --release -p ferrite-cli
```

Timed probe:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world'
```

Prompt tokenization:

- Prompt text: `hello world`
- Prompt token IDs: `[28120, 905]`
- Generated tokens measured: 1
- Output token ID: `30`
- Output token text: `.`
- Thread count: single-threaded Ferrite scalar path

## Results

| Run | Real Time | User Time | Sys Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 0.96 s | 0.36 s | 0.21 s | 770,244,608 bytes | 774,998,848 bytes |
| 2 | 0.47 s | 0.34 s | 0.07 s | 774,684,672 bytes | 774,425,216 bytes |
| 3 | 0.48 s | 0.35 s | 0.07 s | 774,897,664 bytes | 774,687,552 bytes |

Warm-cache summary:

- End-to-end one-token CLI time: 0.47-0.48 s
- Peak RSS: about 775 MB
- Output was stable across all runs: `prompt_token_ids=28120,905`,
  `next_token_id=30`, `next_token=.`

## Interpretation

This baseline proves the current scalar path can run a real Tier 0 135M GGUF
locally without OOM on a 16 GB macOS machine. The memory footprint is much
larger than the 101 MB GGUF file because Ferrite currently reads the full file
and dequantizes tensors into owned F32 scalar matrices.

The result should not be used as a production throughput claim. Ferrite does
not yet have mmap-backed tensor access, persistent model reuse, SIMD kernels,
threading, or a steady-state decode benchmark harness.

## Follow-Up

- Add a steady-state benchmark harness that loads the model once and measures
  repeated next-token calls.
- Add memory accounting that separates GGUF bytes, dequantized weights, KV
  cache, tokenizer metadata, and runtime buffers.
- Compare CPU-only Ferrite against a CPU-only reference runtime after the
  benchmark harness can control backend and threading consistently.
