# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 and Q6_K Benchmark-Token Profile

## Scope

This note records bounded local benchmark-token profiles for the
Qwen2.5-1.5B-Instruct Q8_0 and Q6_K artifacts.

This is profile evidence only. It does not prove the Tier 1 throughput gate and
does not change Q8_K activation-matvec dispatch policy.

## Tree State

- Branch: `main`
- Commit before run: `46eb0e3`
- Working tree before run: clean

The release CLI was rebuilt before profiling:

```sh
cargo build --release -p ferrite-cli
```

## Models

- Q8_0: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
  - SHA-256:
    `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Q6_K: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
  - SHA-256:
    `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7`

Ferrite reported:

```text
q8_0 model_file_bytes=1894532128
q8_0 scalar_weight_bytes=1888581632
q6_k model_file_bytes=1464178720
q6_k scalar_weight_bytes=1458228224
kv_cache_bytes=172032
```

## Protocol

- Host: local macOS aarch64
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Benchmark runs: 1 repeated token-id decode step after initial prompt
  next-token computation
- Profile mode: `--profile-benchmark-token`
- Q8_K activation matvec: disabled

Commands:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 1 \
  --profile-benchmark-token

target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --benchmark-runs 1 \
  --profile-benchmark-token
```

Both runs produced:

```text
next_token_id=198
benchmark_token_ids=9707
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
```

## Summary

| Model | benchmark_avg_ns | profile_total_ns | Approx tok/s from profile | model_file_bytes | scalar_weight_bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Q8_0 | 157,456,917 | 154,193,874 | 6.49 | 1,894,532,128 | 1,888,581,632 |
| Q6_K | 264,085,125 | 222,141,009 | 4.50 | 1,464,178,720 | 1,458,228,224 |

## Role Totals

| Role | Q8_0 ns | Q6_K ns | Observation |
| --- | ---: | ---: | --- |
| `ffn_down` | 38,138,374 | 62,831,708 | Q6_K is slower despite smaller storage |
| `ffn_gate` | 38,133,664 | 52,638,751 | Q6_K is slower |
| `ffn_up` | 38,171,251 | 54,944,209 | Q6_K is slower |
| `q_proj` | 6,560,955 | 15,543,169 | Q6_K is slower |
| `o_proj` | 6,474,919 | 12,654,917 | Q6_K is slower |
| `k_proj` | 1,103,419 | 4,006,502 | Q6_K is slower |
| `v_proj` | 1,102,917 | 4,241,336 | Q6_K is slower |
| `output` | 24,508,375 | 15,280,417 | Q6_K output is faster |

## Interpretation

The profile explains why the local Q8_0 throughput run was faster than the
Q6_K run even though Q8_0 uses larger retained weights. Q6_K saves storage bytes
but is slower across the transformer-layer FFN and projection roles on this
local aarch64 profile.

The exception is the final output projection: Q6_K output was faster than Q8_0
output in this run. That is not enough to offset the larger Q6_K costs in
`ffn_gate`, `ffn_up`, `ffn_down`, `q_proj`, `k_proj`, `v_proj`, and `o_proj`.

The next implementation slice should not assume Q6_K is the better local
throughput target merely because it is smaller on disk. For Qwen2.5-1.5B on
this host, Q8_0 FFN and projection execution is the stronger baseline, while
Q6_K remains useful as a storage and output-projection comparison point.
