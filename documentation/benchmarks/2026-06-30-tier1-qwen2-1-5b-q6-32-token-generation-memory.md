# Tier 1 Qwen2.5-1.5B Q6_K 32-Token Generation Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local release-build 32-token generation probe
for Qwen2.5-1.5B-Instruct Q6_K. It mirrors the Q8_0 32-token generation memory
sample so the two local Qwen2.5-1.5B quantizations have comparable
longer-than-one-token CLI evidence.

This is memory and behavior evidence, not a steady-state throughput claim. It
does not prove longer-context memory posture, long-running RSS stability,
server memory posture, x86_64 behavior, or broader Tier 1 model coverage.

## Environment

- Commit before documentation: `a8644dd`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Binary: `target/release/ferrite`

## Model

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model file bytes: `1464178720`
- Scalar weight bytes: `1458228224`

## Protocol

- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Generated tokens requested: 32
- Execution policy: default only; Q8_K activation matvec disabled
- Memory evidence: `/usr/bin/time -l`

Command:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --generate-tokens 32
```

## Result

Ferrite completed the run with:

```text
prompt_token_ids=14990,1879
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
next_token_id=198
generated_cached_tokens=34
generated_token_ids=198,9707,11,1879,0,2585,646,358,1492,498,3351,30,2160,1052,4113,3151,498,1035,1075,311,1414,476,4263,30,358,2776,1588,311,7789,498,448,894
generated_stopped_on_eos=false
model_file_bytes=1464178720
model_file_retained_bytes=0
scalar_weight_bytes=1458228224
kv_cache_bytes=1949696
11.17 real
23.69 user
6.72 sys
2453192704 maximum resident set size
2963561344 peak memory footprint
327001109024 instructions retired
89252777479 cycles elapsed
```

| Model | Prompt | Generated tokens | Cached tokens | KV cache bytes | Real time | Max RSS | Peak footprint |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5-1.5B Q6_K | `hello world` | 32 | 34 | 1,949,696 | 11.17 s | 2,453,192,704 bytes | 2,963,561,344 bytes |

## Interpretation

The run confirms that the local Qwen2.5-1.5B Q6_K default path can generate the
same 32-token continuation as the Q8_0 artifact from the `hello world` prompt
while retaining 34 cached tokens. It reports the same linear KV-cache footprint
for this model shape: 1,949,696 bytes, or 57,344 bytes per cached token.

The observed max RSS was about 2.45 GB and the peak footprint was about
2.96 GB. That is lower than the corresponding Q8_0 sample because Q6_K has a
smaller weight footprint, but the wall time is higher in this local run. This
remains a bounded local sample: longer contexts, repeated runs, server traffic,
concurrent queue shapes, x86_64 hosts, Q4_K_M, and SmolLM2 still need separate
evidence before the full Tier 1 memory posture can be considered complete.
