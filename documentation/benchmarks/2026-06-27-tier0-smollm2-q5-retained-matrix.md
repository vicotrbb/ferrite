# 2026-06-27 Tier 0 SmolLM2 Q5_0 Retained Matrix

## Scope

This benchmark records the Tier 0 SmolLM2 effect of retaining Q5_0 matrix
bytes in the scalar model and decoding Q5_0 rows on demand during scalar
execution.

This is a memory tradeoff note. It is not an optimized quantized-matmul
throughput claim.

## Tree State

- Branch: `main`
- Commit: `a443702`
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
| 1 | 105,454,432 | 0 | 191,713,536 | 322,560 | 316,686,336 bytes | 312,199,104 bytes |
| 2 | 105,454,432 | 0 | 191,713,536 | 322,560 | 316,686,336 bytes | 311,412,736 bytes |
| 3 | 105,454,432 | 0 | 191,713,536 | 322,560 | 317,112,320 bytes | 311,756,672 bytes |

Timing slowed further relative to Q8_0 retained storage because more matrices
now decode rows on demand:

| Run | benchmark_total_ns | benchmark_avg_ns |
| --- | ---: | ---: |
| 1 | 1,141,180,958 | 228,236,191 |
| 2 | 1,113,454,625 | 222,690,925 |
| 3 | 1,097,232,875 | 219,446,575 |

## Comparison

The prior Q8_0 retained-matrix note at commit `482e987` reported:

- `scalar_weight_bytes=450346752`
- Peak footprint about 569 MB
- Benchmark average about 163-170 ms per accepted token

After retaining Q5_0 matrices:

- `scalar_weight_bytes=191713536`
- Peak footprint about 311-312 MB
- Benchmark average about 219-228 ms per accepted token

The scalar weight accounting dropped by 258,633,216 bytes. Peak footprint
dropped by roughly the same order on this local run. The latency regression is
expected because this scalar reference path decodes compressed matrix rows for
each matrix-vector multiply.

## Interpretation

Ferrite now retains Q8_0 and Q5_0 matrix storage inside the custom scalar
model while preserving deterministic Tier 0 output for the SmolLM2 probe. This
continues moving the scalar loader away from full F32 materialization.

The next memory slices should retain Q4_K and Q6_K matrices, but the matrix
module should be split before adding more quantized row decoders so the code
does not become an oversized mixed-responsibility file.
