# 2026-06-27 Tier 0 SmolLM2 360M Scalar Probe

## Scope

This benchmark records Ferrite's first scalar reference-path memory and latency
probe for the Tier 0 SmolLM2-360M-Instruct Q4_K_M GGUF artifact.

This is not an optimized throughput claim. It measures the current scalar
single-threaded reference path.

## Tree State

- Branch: `main`
- Commit before note: `56112a2`
- Working tree before note: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 16 GB
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/SmolLM2-360M-Instruct-GGUF`
- File: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 270,590,880 bytes
- Hugging Face repo revision recorded by local cache:
  `7be6f65f1db715fe5dc5a4634c0d459b4eed42ec`
- Quantization: Q4_K_M GGUF mixture containing F32, Q8_0, Q5_0, Q4_K, and Q6_K
  tensors

## Commands

Build:

```sh
cargo build --release -p ferrite-cli
```

Repeated-token timed probe:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Prompt and output:

- Prompt text: `hello world`
- Prompt token IDs: `[28120, 905]`
- Initial output token ID: `18`
- Initial output token text: `"`
- Benchmark token IDs: `[284, 476, 28120, 905, 18]`
- Cached tokens after benchmark loop: 7
- Thread count: single-threaded Ferrite scalar path

## Results

| scalar_weight_bytes | kv_cache_bytes | benchmark_total_ns | benchmark_avg_ns | Real Time | User Time | Max RSS | Peak Footprint |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 268,803,840 | 573,440 | 2,770,622,750 | 554,124,550 | 4.24 s | 3.89 s | 554,369,024 bytes | 554,650,048 bytes |

Full output:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_token_ids=284,476,28120,905,18
benchmark_total_ns=2770622750
benchmark_avg_ns=554124550
model_file_bytes=270590880
model_file_retained_bytes=0
scalar_weight_bytes=268803840
kv_cache_bytes=573440
        4.24 real         3.89 user         0.12 sys
           554369024  maximum resident set size
           554650048  peak memory footprint
```

## Interpretation

The 360M model stays within local-memory safety bounds for a short Tier 0 probe
and exercises the same Llama-family parser, tokenizer, quantized matrix, GQA,
KV cache, generation, and streaming surfaces as the smaller 135M model with
larger dimensions.

The scalar reference path is still far from Tier 1 throughput goals: this 360M
run averaged about 554 ms per repeated accepted token on the local Apple M1 Pro.
Before moving to Tier 1 performance claims, Ferrite still needs explicit
SIMD-kernel work and stricter reference parity investigation for the 360M
CPU-only `llama.cpp` path.
