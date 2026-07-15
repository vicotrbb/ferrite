# Bounded Embedding Row Decode Diagnostic

Date: 2026-07-14

## Scope

This diagnostic verifies one memory-allocation correction. Q4_K, Q5_K, and
Q6_K token embedding lookup now decodes only the quantization blocks that
intersect the requested row. It does not decode the complete matrix and then
discard every other row.

The same implementation snapshot also retains mapped F16 and BF16 matrices and
converts them during matvec. Neither evaluated model uses F16 or BF16 matrix
weights, so the real-model result below isolates bounded quantized row access.

## Fixed Inputs

- Host: Apple M5 Pro, 15 cores, 24 GiB RAM
- OS: macOS 26.5.2 arm64
- Toolchain: Rust 1.96.0
- Source base: commit `33a11d0be6e2417a145d9aea5033a6430be4163d`
  on `main`, with the evaluated implementation in a dirty working tree
- Cargo profile: repository `release` profile, no `RUSTFLAGS`
- CLI SHA-256: `c4bdd045b57e91ecc9841c283f200478e2e8c2bbd04d3c8fc408fa0460bedfd7`
- Server SHA-256: `3dfd722c9bcf72bb1f7e523665bd5478dab876aa2b5ada9e7cda949c2ea4e552`
- Throughput client SHA-256:
  `69efa77f0f98a70cb099e5f69cb2ae88ea237ce9cd3b09d2b3658f666af8f85e`
- SmolLM2 model SHA-256:
  `decd2598bc2c8ed08c19adc3c8fdd461ee19ed5708679d1c54ef54a5a30d4f33`
- Phi-3 model SHA-256:
  `8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef`
- Prompt: `Write a short story about a rusty robot who learns to sail.`
- Output budget: 64 tokens
- Requests: four per measured cohort
- Soak: three rounds, 2,000 ms idle delay, 16 MiB tolerance
- Server policy: automatic kernels, Locus KV, 16 tokens per block, 128-token
  per-session cap

The host preflight rejected unrelated background load. This is therefore a
correctness and allocation diagnostic, not clean performance evidence.

## Root Cause

The two token embedding shapes explain the transient allocation exactly:

```text
SmolLM2: 49,152 rows * 2,048 columns * 4 F32 bytes = 402,653,184 bytes
Phi-3:   32,064 rows * 3,072 columns * 4 F32 bytes = 394,002,432 bytes
```

The old row accessor decoded that full F32 value count for every token lookup.
Allocator purge timing made idle samples appear stable in some cohorts and
unstable in others. The new accessor validates full storage, decodes the
minimal intersecting block window, and copies only the selected row.

## Command

```sh
python3 scripts/eval.py \
  --model target/models/smollm2-1.7b-instruct-q4_k_m.gguf \
  --model /path/to/Phi-3-mini-4k-instruct-q4.gguf \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --skip-cli \
  --server-batch-streams 4 \
  --requests 4 \
  --server-workload identical \
  --server-soak-rounds 3 \
  --server-soak-idle-ms 2000 \
  --server-soak-rss-tolerance-mib 16 \
  --server-kv-backend locus \
  --server-kv-tokens-per-block 16 \
  --server-kv-max-tokens 128 \
  --tag diagnostic-bounded-quantized-rows-locus-soak \
  --skip-build
```

The pre-fix artifact used the same model pair, prompt, token count, requests,
soak rounds, idle delay, tolerance, and Locus settings. Its tag differed because
the initial dense-16 hypothesis had not yet identified quantized row lookup as
the active allocation.

## Result

| Model and route | Pre-fix gate | Pre-fix physical-footprint tail range | Post-fix gate | Post-fix physical-footprint tail range | Pre-fix peak RSS | Post-fix peak RSS |
| --- | --- | ---: | --- | ---: | ---: | ---: |
| SmolLM2 default | pass | 32,768 B | pass | 1,196,032 B | 1,466.9 MiB | 1,083.1 MiB |
| SmolLM2 batched 4 | pass | 606,208 B | pass | 3,080,192 B | 1,593.6 MiB | 1,223.8 MiB |
| Phi-3 default | fail | 397,771,040 B | pass | 835,584 B | 2,728.7 MiB | 2,312.1 MiB |
| Phi-3 batched 4 | fail | 394,625,312 B | pass | 3,784,704 B | 2,914.8 MiB | 2,480.4 MiB |

All post-fix physical-footprint tail ranges are below the fixed 16,777,216-byte
limit. Default and batched token traces were stable in every soak round, and
each batched trace matched its default-route trace.

The pre-fix artifact is
[`2026-07-14-122938`](../../scripts/evals/2026-07-14-122938-smollm2-1.7b-instruct-q4_k_m-multi.md).
The post-fix artifact is
[`2026-07-14-130458`](../../scripts/evals/2026-07-14-130458-smollm2-1.7b-instruct-q4_k_m-multi.md).

A later acceptance attempt exposed a separate sampling issue: Phi-3 continuous
batching made one bounded private-footprint step between the first and second
soak samples, then held a 98,304-byte tail range. The evaluator now runs one
unmeasured exact cohort before collecting idle samples. The focused
[`2026-07-14-145254`](../../scripts/evals/2026-07-14-145254-phi-3-mini-4k-instruct-q4.md)
diagnostic passed the unchanged 16 MiB gate with 5,832,704 bytes of growth and a
5,718,016-byte tail range. The rejected
[`2026-07-14-143042`](../../scripts/evals/2026-07-14-143042-qwen2.5-1.5b-instruct-q8_0-multi.md)
artifact remains retained. Neither run is used as clean-host performance
evidence.

Independent CLI checks also reproduced the saved pre-change 64-token benchmark
trace for both models under automatic dispatch. Portable-provider checks and
dense-16 unit tests retained exact accumulation results.

## Acceptance

Accepted as a bounded-allocation and correctness fix. The host was not clean,
so TTFT, throughput, CPU, and peak-RSS differences remain diagnostic and are
not promoted as performance claims. A separate clean repeated acceptance suite
is required for those claims.
