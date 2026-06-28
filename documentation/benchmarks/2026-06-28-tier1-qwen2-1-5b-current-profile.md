# 2026-06-28 Tier 1 Qwen2.5 1.5B Current Profile

## Scope

This benchmark records a fresh current Qwen2.5-1.5B-Instruct Q4_K_M
benchmark-token profile after the CLI EOS generation stop slice.

This is evidence only. It does not change runtime code and does not prove the
Tier 1 throughput target.

## Tree State

- Branch: `main`
- Commit: `29e880a`
- Working tree before run: clean

## Model

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- File: `qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local size reported by Ferrite: 1,117,320,736 bytes
- Scalar weight bytes reported by Ferrite: 1,111,370,240 bytes

## Protocol

- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Initial next token ID: `198`
- Benchmark runs: 3 repeated token-id decode steps after initial prompt
  next-token computation
- Profiled benchmark token input: `198`
- Profiled benchmark token output: `9707`
- Thread setting: default Rayon pool
- Memory evidence: `/usr/bin/time -l`

Command:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
```

## Result

| benchmark_avg_ns | Approx tok/s | profile_total_ns | Real time | User time | Max RSS | Peak footprint |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 296,499,736 | 3.37 | 121,382,921 | 4.22 s | 4.79 s | 2,091,286,528 bytes | 2,268,550,976 bytes |

Aggregate profile roles:

```text
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:12245749
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:17353041
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:26667421
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:27393334
profile_benchmark_token_role=k_proj:Q4_K:256:1536:221184:3163499
profile_benchmark_token_role=o_proj:Q4_K:1536:1536:1327104:6949748
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:17050875
profile_benchmark_token_role=q_proj:Q4_K:1536:1536:1327104:7427421
profile_benchmark_token_role=v_proj:Q4_K:256:1536:221184:1453541
profile_benchmark_token_role=v_proj:Q6_K:256:1536:322560:1678292
```

Memory and model summary:

```text
model_file_bytes=1117320736
model_file_retained_bytes=0
scalar_weight_bytes=1111370240
kv_cache_bytes=286720
```

## Interpretation

The current Qwen2.5-1.5B profile remains below the Tier 1 throughput target.
The profile is still dominated by Q4_K FFN gate/up work, Q4_K/Q6_K FFN down
work, and the Q6_K output projection. Together, Q4_K FFN gate/up account for
54,060,755 ns of the profiled token, while Q4_K/Q6_K FFN down account for
29,598,790 ns and Q6_K output accounts for 17,050,875 ns.

The next optimization should remain focused on Q4_K/Q6_K kernel efficiency or a
different decode scheduling hypothesis. The already tested Q4_K/Q6_K thresholded
row-parallel scheduling and Q6_K `try_reduce` argmax shapes should not be
repeated without a different hypothesis.
