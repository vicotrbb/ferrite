# Tier 1 SmolLM2-1.7B Q4_K_M 32-Token Generation Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local release-build 32-token generation probe
for SmolLM2-1.7B-Instruct Q4_K_M. It expands the longer-than-one-token memory
evidence beyond Qwen2.5-1.5B and adds a second Tier 1 model family to the local
CLI generation-memory set.

This is memory and behavior evidence, not a steady-state throughput claim. It
does not prove longer-context memory posture, long-running RSS stability,
server memory posture, x86_64 behavior, or broader Tier 1 model coverage.

## Environment

- Commit before documentation: `77f81ae`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Binary: `target/release/ferrite`

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Model file bytes: `1055609824`
- Scalar weight bytes: `1053827072`

## Protocol

- Prompt: `hello world`
- Prompt token IDs: `28120,905`
- Generated tokens requested: 32
- Execution policy: default only; Q8_K activation matvec disabled
- Memory evidence: `/usr/bin/time -l`

Command:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --prompt 'hello world' \
  --generate-tokens 32
```

## Result

Ferrite completed the run with:

```text
prompt_token_ids=28120,905
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
next_token_id=18
generated_cached_tokens=34
generated_token_ids=18,198,3725,198,198,788,451,1183,28,260,2179,28120,80,1517,314,4355,351,253,2244,8709,2179,1245,9838,527,314,4180,288,260,2179,3272,79,28120
generated_stopped_on_eos=false
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=13369344
6.93 real
19.60 user
4.66 sys
1690337280 maximum resident set size
2123551744 peak memory footprint
225421240603 instructions retired
70800522985 cycles elapsed
```

| Model | Prompt | Generated tokens | Cached tokens | KV cache bytes | Real time | Max RSS | Peak footprint |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| SmolLM2-1.7B Q4_K_M | `hello world` | 32 | 34 | 13,369,344 | 6.93 s | 1,690,337,280 bytes | 2,123,551,744 bytes |

## Interpretation

The run confirms that the local SmolLM2-1.7B Q4_K_M default path can generate
32 tokens from the `hello world` prompt while retaining 34 cached tokens. Its
reported KV-cache footprint was 13,369,344 bytes, or 393,216 bytes per cached
token, much larger than the 57,344 bytes per cached token observed for the
Qwen2.5-1.5B Q8_0/Q6_K samples.

The observed max RSS was about 1.69 GB and the peak footprint was about
2.12 GB. This adds cross-family longer-generation memory evidence for Tier 1,
but it remains a bounded local sample. Longer contexts, repeated runs, server
traffic, concurrent queue shapes, x86_64 hosts, and other quantizations still
need separate evidence before the full Tier 1 memory posture can be considered
complete.
