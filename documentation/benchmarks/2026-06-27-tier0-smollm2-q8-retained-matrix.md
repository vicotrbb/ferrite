# 2026-06-27 Tier 0 SmolLM2 Q8_0 Retained Matrix

## Scope

This benchmark records the Tier 0 SmolLM2 effect of retaining Q8_0 matrix
bytes in the scalar model and decoding Q8_0 rows on demand during scalar
execution.

This is a memory tradeoff note. It is not an optimized quantized-matmul
throughput claim.

## Tree State

- Branch: `main`
- Commit: `482e987`
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
| 1 | 105,454,432 | 0 | 450,346,752 | 322,560 | 571,228,160 bytes | 569,575,744 bytes |
| 2 | 105,454,432 | 0 | 450,346,752 | 322,560 | 574,980,096 bytes | 569,215,232 bytes |
| 3 | 105,454,432 | 0 | 450,346,752 | 322,560 | 575,062,016 bytes | 569,395,456 bytes |

Timing slowed relative to the previous scalar-session range because Q8_0 rows
are now decoded during scalar matvecs:

| Run | benchmark_total_ns | benchmark_avg_ns |
| --- | ---: | ---: |
| 1 | 815,046,125 | 163,009,225 |
| 2 | 851,626,333 | 170,325,266 |
| 3 | 839,002,834 | 167,800,566 |

## Comparison

The prior retained-model-byte note at commit `dc8398f` reported:

- `scalar_weight_bytes=538060032`
- Peak footprint about 657 MB
- Benchmark average about 108-114 ms per accepted token

After retaining Q8_0 matrices:

- `scalar_weight_bytes=450346752`
- Peak footprint about 569 MB
- Benchmark average about 163-170 ms per accepted token

The scalar weight accounting dropped by 87,713,280 bytes. Peak footprint
dropped by roughly the same order on this local run. The latency regression is
expected for this scalar reference implementation because Q8_0 rows are decoded
for each matrix-vector multiply instead of being pre-expanded.

## Interpretation

This proves Ferrite can retain at least one quantized matrix format inside the
custom scalar model while preserving deterministic Tier 0 token output. The
next quantized-storage slices should extend the same representation boundary to
Q5_0, Q4_K, and Q6_K, then replace row decode plus F32 dot with direct
quantized scalar matvec kernels to recover latency.
