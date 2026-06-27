# 2026-06-27 Tier 0 SmolLM2 Q4_K Retained Matrix

## Scope

This benchmark records the Tier 0 SmolLM2 effect of retaining Q4_K matrix
bytes in the scalar model.

This is a memory tradeoff note. It is not an optimized quantized-matmul
throughput claim.

## Tree State

- Branch: `main`
- Commit: `6ffb12c`
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
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world'
```

Prompt and output:

- Prompt text: `hello world`
- Prompt token IDs: `[28120, 905]`
- Initial output token ID: `30`
- Initial output token text: `.`
- Cached tokens after prompt: 2
- Thread count: single-threaded Ferrite scalar path

## Results

| Run | model_file_bytes | model_file_retained_bytes | scalar_weight_bytes | kv_cache_bytes | Real Time | User Time | Sys Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 105,454,432 | 0 | 143,053,056 | 92,160 | 14.26 s | 12.30 s | 1.28 s | 288,899,072 bytes | 263,423,808 bytes |

## Comparison

The prior Q5_0 retained-matrix note at commit `a443702` reported:

- `scalar_weight_bytes=191713536`
- Peak footprint about 311-312 MB
- Repeated benchmark average about 219-228 ms per accepted token

After retaining Q4_K matrices:

- `scalar_weight_bytes=143053056`
- Peak footprint about 263 MB for the single-token probe
- End-to-end single-token probe time was 14.26 s

The scalar weight accounting dropped by 48,660,480 bytes relative to the Q5_0
retained baseline. The cost is a severe scalar compute regression because the
current Q4_K row path decodes a temporary full matrix for each requested row.

## Interpretation

Ferrite now retains Q8_0, Q5_0, and Q4_K matrix storage inside the custom
scalar model while preserving deterministic Tier 0 output for the SmolLM2
probe. The memory result is useful, but the scalar Q4_K execution path is not
acceptable as a steady-state decode path.

The next slice should implement a direct or cached Q4_K scalar matvec path so
Q4_K retained storage does not require repeated full-matrix decode. Q6_K
retention should wait until this Q4_K latency problem is bounded or isolated.
