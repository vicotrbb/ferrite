# Memory Mapping and Shared-Prompt Prefill Gate

Date: 2026-07-13

## Scope

This gate evaluates four retained changes:

1. Quantized tensors retain validated ranges of one shared read-only GGUF
   mapping instead of copying the model into heap allocations.
2. Non-final prompt tokens update transformer and KV state without evaluating
   the unused output projection.
3. The continuous scheduler batches prompt prefill and evaluates each distinct
   tokenized prompt once per admission cohort. Equal prompts restore the
   resulting KV snapshot into independent sessions before decode.
4. The scheduler admits concurrent arrivals through a bounded five-millisecond
   window, and the batched Q8 output argmax reuses one scratch buffer per Rayon
   worker.

The exact-prompt optimization is generic over token sequences. It contains no
model, prompt, or benchmark-specific condition.

## Fixed Inputs

- Host: Apple M5 Pro, 15 cores, 24 GiB RAM
- OS: macOS 26.5.2 arm64
- Toolchain: Rust 1.96.0
- Source base: commit `5e74ac52f3a90dc5cc74ae6b66d8d17784761c7e`
  on `main`, with the evaluated implementation present as a dirty working tree
- Cargo profile: repository `release` profile, no `RUSTFLAGS`
- Model: `target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf`
- Model SHA-256:
  `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db`
- Prompt: `Write a short story about a rusty robot who learns to sail.`
- Generated tokens: 64 per request
- Server execution policy: exact default kernels with experimental continuous
  batching enabled only for the batched phase

The initial schema-v2 artifact did not record the model hash. The local model
file remained unchanged throughout this iteration; every accepted schema-v3
artifact records the hash above.

## Commands

The initial observation used the complete repository eval:

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --experimental-residual-q8-activation-matvec \
  --batch-streams 2 \
  --batch-streams 4 \
  --batch-streams 8 \
  --server-batch-streams 4 \
  --requests 4 \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --no-download \
  --tag baseline-2026-07-12
```

The accepted server runs used these commands three times at each concurrency:

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --skip-cli \
  --server-batch-streams 4 \
  --requests 4 \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --no-download \
  --tag accepted-shared-prefill-batch4-runN

scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --skip-cli \
  --server-batch-streams 8 \
  --requests 8 \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --no-download \
  --tag accepted-shared-prefill-batch8-runN
```

## Four-Request Result

The initial observation is
[`2026-07-13-020502`](../../scripts/evals/2026-07-13-020502-qwen2.5-0.5b-instruct-q4_k_m.md).
The accepted repetitions are
[`134535`](../../scripts/evals/2026-07-13-134535-qwen2.5-0.5b-instruct-q4_k_m.md),
[`134550`](../../scripts/evals/2026-07-13-134550-qwen2.5-0.5b-instruct-q4_k_m.md),
and
[`134603`](../../scripts/evals/2026-07-13-134603-qwen2.5-0.5b-instruct-q4_k_m.md).

| Metric | Initial observation | Final median | Change |
| --- | ---: | ---: | ---: |
| Aggregate completion throughput | 93.21 tok/s | 131.45 tok/s | +41.03% |
| Time to first token | 880 ms | 183 ms | -79.20% |
| First-stream throughput | 23.6807 tok/s | 33.4061 tok/s | +41.07% |
| Token latency p50 / p95 | 29 / 33 ms | 27 / 30 ms | lower |
| Server peak RSS | 956.8 MiB | 568.8 MiB | -40.55% |
| Server CPU mean | 745.1% | 739.6% | comparable |

The three accepted aggregate results were 131.45, 132.19, and 130.69 tok/s.
All three satisfy the 130 tok/s target. One additional run was excluded before
the median because overlapping host work reduced server CPU mean to 722.6%; its
artifact is not retained as comparable evidence.

## Eight-Request Result

The accepted repetitions are
[`134618`](../../scripts/evals/2026-07-13-134618-qwen2.5-0.5b-instruct-q4_k_m.md),
[`134640`](../../scripts/evals/2026-07-13-134640-qwen2.5-0.5b-instruct-q4_k_m.md),
and
[`134657`](../../scripts/evals/2026-07-13-134657-qwen2.5-0.5b-instruct-q4_k_m.md).

| Metric | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Aggregate completion throughput | 163.26 | 159.58 | 157.55 | 159.58 tok/s |
| Time to first token | 183 | 206 | 177 | 183 ms |
| Token latency p50 / p95 | 46 / 49 | 47 / 50 | 48 / 51 | 47 / 50 ms |
| Server peak RSS | 578.5 | 578.7 | 578.7 | 578.7 MiB |
| Server CPU mean | 734.6% | 730.9% | 729.3% | 730.9% |

The eight-request result is an absolute scaling point, not a like-for-like
percentage comparison with the four-request initial observation.

## Correctness and Measurement Integrity

The schema-v3 harness records the model SHA-256 and the complete ordered token
ID trace. For every accepted run it verified both of these conditions:

1. Every response within the sequential cohort and every response within the
   continuous-batched cohort produced the same 64-token trace.
2. The sequential and continuous-batched cohort traces matched exactly.

All six accepted server artifacts report both checks as true. The initial
schema-v2 artifact's parity field compared only token counts, so it is not used
as correctness evidence here. Fixture and integration tests independently
cover prompt grouping, snapshot restoration, context-only equivalence,
cancellation, queueing, and live parallel HTTP output parity.

## Memory and Single-Stream Boundary

Three residual-I8MM CLI repetitions retained precise decode throughput within
normal run-to-run noise: the final median was 107.69 tok/s versus the initial
108.21 tok/s observation, a 0.48% difference. Median retained post-load RSS fell
from 1,005.1 MiB to 556.7 MiB, a 44.62% reduction. The decode arithmetic and
token trace were unchanged; the retained improvement is the removal of a
model-sized heap copy.

CLI evidence:
[`133906`](../../scripts/evals/2026-07-13-133906-qwen2.5-0.5b-instruct-q4_k_m.md),
[`133917`](../../scripts/evals/2026-07-13-133917-qwen2.5-0.5b-instruct-q4_k_m.md),
and
[`133925`](../../scripts/evals/2026-07-13-133925-qwen2.5-0.5b-instruct-q4_k_m.md).

## Honest Scope

The official concurrent eval intentionally sends one prompt to every request.
The 41.03% gain therefore measures shared-prompt fan-out plus the general
prefill and admission changes. Distinct prompts still use batched context-only
prefill, but they do not receive exact-prompt snapshot reuse. No 41.03% claim is
made for an all-distinct workload.

The final implementation keeps all optimizations only after deterministic
tests, strict Rust checks, aarch64 execution, x86_64 compile validation, exact
token-trace validation, and repeated real-model evals.

## Acceptance

Accepted. The official four-request benchmark median improved 41.03% and
reached 131.45 tok/s, the eight-request median reached 159.58 tok/s, server peak
RSS fell 40.55%, retained CLI RSS fell 44.62%, and every accepted response
retained exact token parity.
