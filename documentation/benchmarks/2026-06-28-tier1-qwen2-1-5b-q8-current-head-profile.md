# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 Current-Head Profile

## Scope

This note records a fresh local benchmark-token profile for
Qwen2.5-1.5B-Instruct Q8_0 at current head after the OpenAI server proof
slices. No inference code changed in those HTTP slices, but this refreshes the
profile handle before the next throughput optimization.

This is evidence only. It does not prove the Tier 1 throughput target.

## Tree State

- Branch: `main`
- Commit: `cecf6e4`
- Working tree before run: clean
- Release CLI rebuilt before profiling:

```sh
cargo build --release -p ferrite-cli
```

## Model

- File: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- SHA-256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Local size: 1.8 GiB by `ls -lh`

Ferrite reported:

```text
model_file_bytes=1894532128
model_file_retained_bytes=0
scalar_weight_bytes=1888581632
kv_cache_bytes=286720
```

## Protocol

- Host: local macOS aarch64
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Initial next token ID: `198`
- Benchmark runs: 3 repeated token-id decode steps after initial prompt
  next-token computation
- Profiled benchmark token input: `198`
- Profiled benchmark token output: `9707`
- Q8_K activation matvec: disabled

Command:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 3 \
  --profile-benchmark-token
```

## Result

| benchmark_avg_ns | Approx tok/s | profile_total_ns | Real time | User time | Sys time | Max RSS | Peak footprint |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 155,274,902 | 6.44 | 155,224,492 | 5.00 s | 1.45 s | 1.52 s | 1,671,757,824 bytes | 3,823,264,256 bytes |

Token output:

```text
benchmark_token_ids=9707,11,1879
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
q8_k_activation_matvec_policy=default_only
q8_k_activation_matvec_roles=all
```

Aggregate profile roles:

```text
profile_benchmark_token_role=ffn_down:Q8_0:1536:8960:14622720:38526292
profile_benchmark_token_role=ffn_gate:Q8_0:8960:1536:14622720:38553788
profile_benchmark_token_role=ffn_up:Q8_0:8960:1536:14622720:38131624
profile_benchmark_token_role=k_proj:Q8_0:256:1536:417792:1194585
profile_benchmark_token_role=o_proj:Q8_0:1536:1536:2506752:6722458
profile_benchmark_token_role=output:Q8_0:151936:1536:247959552:24297958
profile_benchmark_token_role=q_proj:Q8_0:1536:1536:2506752:6684330
profile_benchmark_token_role=v_proj:Q8_0:256:1536:417792:1113457
```

## Interpretation

The current-head Q8_0 profile remains below the 10 tok/s Tier 1 target. The
largest retained roles are still the Q8_0 FFN matrices plus the final output
projection:

- `ffn_gate`, `ffn_up`, and `ffn_down` together account for 115,211,704 ns.
- `output` accounts for 24,297,958 ns.
- Q/K/V/O projections account for 15,714,830 ns combined.

The next optimization should not repeat the already rejected naive Q8_0
row-parallel scheduling shape. A better next hypothesis needs to target Q8_0
FFN block-dot efficiency, final-output argmax scheduling, or a more selective
parallelism threshold that is validated against this current-head profile.
