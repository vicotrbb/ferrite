# 2026-06-27 Tier 0 SmolLM2 Session-Cache Scalar

## Scope

This benchmark records Ferrite's first Tier 0 scalar benchmark using
incremental `ScalarLlamaSession` cache reuse. The CLI loads the model once,
accepts the prompt once, then `--benchmark-runs` repeatedly accepts the
previously generated token with cached per-layer K/V state.

This is still a scalar reference benchmark, not an optimized production decode
benchmark.

## Tree State

- Branch: `main`
- Commit: `c38a650`
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
- Cached tokens after benchmark loop: 7
- Thread count: single-threaded Ferrite scalar path

## Results

| Run | benchmark_total_ns | benchmark_avg_ns | Real Time | User Time | Sys Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 579,795,375 | 115,959,075 | 1.49 s | 0.91 s | 0.18 s | 772,325,376 bytes | 775,801,536 bytes |
| 2 | 556,707,084 | 111,341,416 | 1.15 s | 0.89 s | 0.10 s | 775,340,032 bytes | 775,375,616 bytes |
| 3 | 537,224,334 | 107,444,866 | 1.03 s | 0.86 s | 0.10 s | 775,651,328 bytes | 775,391,936 bytes |

Summary:

- Incremental scalar session next-token time: 107.4-116.0 ms per accepted
  token.
- Approximate incremental scalar session rate: 8.6-9.3 accepted tokens/s.
- Peak RSS: about 775 MB.
- Output was stable across all runs: `prompt_token_ids=28120,905`,
  `next_token_id=30`, `next_token=.`, `benchmark_cached_tokens=7`.

## Comparison

The previous repeated full-prompt scalar baseline at commit `f821d55` measured
about 212.9-224.0 ms per call for the same prompt and model. This session-cache
baseline removes prompt replay inside the benchmark loop and measures
incremental accepted-token execution.

This is not a general speedup claim for production inference. It is a local
baseline for the scalar reference path on this hardware and model.

## Follow-Up

- Add generated-token output for benchmark loops so multi-token sequences can
  be compared against a reference runtime.
- Add component memory accounting for model bytes, F32 weights, tokenizer
  metadata, K/V cache, and temporary buffers.
- Add optimized quantized matmul and compare it against this scalar session
  baseline.
