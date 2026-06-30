# Tier 1 Qwen2.5-1.5B Q8_0 Second-Prompt Throughput

Date: 2026-06-30

## Scope

This benchmark records a bounded local release-build throughput sample for
Qwen2.5-1.5B-Instruct Q8_0 on Ferrite's second fixed Qwen2 prompt,
`The capital of France is`.

This expands local prompt coverage for the Q8_0 path that already had
`hello world` evidence above 10 tok/s. It does not complete the Tier 1
throughput gate: broader prompts, Q6_K/Q4_K_M, SmolLM2, x86_64, longer
generations, and full-tier steady-state behavior remain open.

## Environment

- Commit: `5043fcd`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Command used to build: `cargo build --release -p ferrite-cli`

Build result:

```text
Finished `release` profile [optimized] target(s) in 2.14s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q8_0 GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model file bytes: `1894532128`
- Scalar weight bytes: `1888581632`

## Protocol

- Prompt: `The capital of France is`
- Prompt token IDs: `785,6722,315,9625,374`
- Expected next token ID: `12095`
- Benchmark runs: 5 repeated token-id decode steps after the initial prompt
  next-token computation
- Execution policy: default only; Q8_K activation matvec disabled
- Memory evidence: `/usr/bin/time -l`

Command:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'The capital of France is' \
  --benchmark-runs 5 \
  --expect-token-id 12095
```

## Result

Ferrite matched the expected next token and reported:

```text
prompt_token_ids=785,6722,315,9625,374
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
next_token_id=12095
benchmark_runs=5
benchmark_cached_tokens=10
benchmark_token_ids=13,576,6722,315,9625
benchmark_total_ns=433899417
benchmark_avg_ns=86779883
model_file_bytes=1894532128
model_file_retained_bytes=0
scalar_weight_bytes=1888581632
kv_cache_bytes=573440
expected_token_id=12095
match=true
3.84 real
2.23 user
1.86 sys
3133669376 maximum resident set size
3823247680 peak memory footprint
44405088675 instructions retired
12190918350 cycles elapsed
```

| Model | Prompt | benchmark_avg_ns | Approx tok/s | Real time | Max RSS | Peak footprint |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5-1.5B Q8_0 | `The capital of France is` | 86,779,883 | 11.52 | 3.84 s | 3,133,669,376 bytes | 3,823,247,680 bytes |

## Interpretation

The local default-pool Qwen2.5-1.5B Q8_0 path stayed above the 10 tok/s
single-model benchmark-token threshold on a second fixed Qwen2 prompt. The
observed rate, about 11.52 tok/s, is slower than the earlier `hello world`
post-optimization sample of about 12.15 tok/s but still above threshold on this
local Apple M1 Pro host.

This is useful prompt-coverage evidence for the optimized Q8_0 path. It does
not change the full Tier 1 verdict: Q6_K, Q4_K_M, SmolLM2-1.7B, x86_64
throughput, longer generations, memory under longer runs, and broader HTTP
throughput remain separate gates.
