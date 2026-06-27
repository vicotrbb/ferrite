# 2026-06-27 Tier 0 SmolLM2 Steady-State Scalar

## Scope

This benchmark uses Ferrite's `--benchmark-runs` CLI mode to measure repeated
in-process scalar `next_token_for_prompt` calls after the model has already
been loaded and the initial next token has already been printed.

The benchmark still runs through the CLI and does not yet separate tokenizer,
prompt setup, or process startup from the full command. The reported
`benchmark_avg_ns` values measure only the repeated next-token loop.

## Tree State

- Branch: `main`
- Commit: `f821d55`
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
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Prompt and output:

- Prompt text: `hello world`
- Prompt token IDs: `[28120, 905]`
- Initial output token ID: `30`
- Initial output token text: `.`
- Repeated benchmark calls per process: 5
- Thread count: single-threaded Ferrite scalar path

## Results

| Run | benchmark_total_ns | benchmark_avg_ns | Real Time | User Time | Sys Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1,064,374,458 | 212,874,891 | 2.02 s | 1.39 s | 0.21 s | 768,245,760 bytes | 775,162,624 bytes |
| 2 | 1,120,065,500 | 224,013,100 | 1.57 s | 1.39 s | 0.10 s | 776,110,080 bytes | 775,916,288 bytes |
| 3 | 1,067,837,792 | 213,567,558 | 1.52 s | 1.37 s | 0.08 s | 775,340,032 bytes | 775,080,640 bytes |

Summary:

- Repeated scalar next-token time: 212.9-224.0 ms per call.
- Approximate repeated scalar next-token rate: 4.5-4.7 calls/s.
- Peak RSS: about 775 MB.
- Output was stable across all runs: `prompt_token_ids=28120,905`,
  `next_token_id=30`, `next_token=.`

## Interpretation

This is the first steady-state scalar baseline for a real Tier 0 model in
Ferrite. It is still a scalar reference path, not an optimized CPU inference
engine. The result is useful as a regression baseline for future SIMD,
threading, mmap, and KV-cache reuse work.

The benchmark repeats the same full prompt path each time. It does not yet
measure autoregressive multi-token generation with incremental KV reuse.

## Follow-Up

- Add an API-level benchmark harness that loads once and avoids CLI output.
- Add incremental KV-cache reuse so repeated token generation does not replay
  the whole prompt.
- Add component memory accounting for GGUF bytes, F32 weights, tokenizer
  metadata, KV cache, and temporary buffers.
