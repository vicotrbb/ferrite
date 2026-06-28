# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_K Opt-In Benchmark

## Scope

This benchmark records whether the opt-in Q4_K/Q6_K x Q8_K activation matvec
path improves local Qwen2.5-1.5B-Instruct Q4_K_M decode throughput.

This is evidence only. It does not promote Q8_K activation matvecs to default
dispatch. The path still fails SmolLM2-1.7B multi-token parity, so default
eligibility remains blocked by correctness, not by benchmark value.

## Tree State

- Branch: `main`
- Commit: `ec8417c`
- Working tree before run: clean

## Model

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- File: `qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local size reported by Ferrite: 1,117,320,736 bytes
- Scalar weight bytes reported by Ferrite: 1,111,370,240 bytes

## Protocol

- Prompt: `hello world`
- Benchmark runs: 3 repeated token-id decode steps after initial prompt
  next-token computation
- Profile: `--profile-benchmark-token`
- Memory evidence: `/usr/bin/time -l`

Commands:

```sh
cargo build --release -p ferrite-cli
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token --experimental-q8-k-activation-matvec
/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token --experimental-q8-k-activation-matvec
```

## Results

All four runs produced the same benchmark token IDs:

```text
benchmark_token_ids=9707,11,1879
```

| Thread setting | Q8_K flag | benchmark_avg_ns | Approx tok/s | profile_total_ns | Real time | User time | Max RSS | Peak footprint |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| default pool | off | 261,316,083 | 3.83 | 175,834,213 | 4.09 s | 4.80 s | 2,097,545,216 bytes | 2,268,206,720 bytes |
| default pool | on | 226,673,736 | 4.41 | 61,006,374 | 3.24 s | 2.66 s | 2,076,344,320 bytes | 2,268,321,280 bytes |
| `RAYON_NUM_THREADS=2` | off | 383,523,500 | 2.61 | 228,656,622 | 4.77 s | 4.02 s | 2,097,627,136 bytes | 2,268,239,488 bytes |
| `RAYON_NUM_THREADS=2` | on | 263,642,694 | 3.79 | 96,380,203 | 3.36 s | 2.10 s | 2,076,573,696 bytes | 2,267,944,448 bytes |

## Profile Details

Default-pool baseline:

```text
profile_benchmark_token_total_ns=175834213
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:21946416
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:27005335
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:40698626
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:39498669
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:17343167
```

Default-pool Q8_K opt-in:

```text
profile_benchmark_token_total_ns=61006374
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:6536959
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:7755293
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:12382749
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:13017916
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:6361167
```

Two-thread baseline:

```text
profile_benchmark_token_total_ns=228656622
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:25573916
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:34330292
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:49817459
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:54272207
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:39076250
```

Two-thread Q8_K opt-in:

```text
profile_benchmark_token_total_ns=96380203
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:10906292
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:13870251
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:21362959
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:21273043
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:16203042
```

## Interpretation

Path B is approved as a sound opt-in kernel-contract experiment. The arithmetic
audit in `documentation/dev-notes/2026-06-28-q8-k-reference-arithmetic.md`
found no Q4_K/Q6_K x Q8_K formula hole against the local `llama.cpp` generic and
ARM NEON contracts, and this benchmark proves that the route has real Qwen2.5
1.5B performance value.

For Qwen2.5-1.5B, the opt-in route improved the default-pool benchmark average
from 261,316,083 ns to 226,673,736 ns, about 13.3 percent. With
`RAYON_NUM_THREADS=2`, it improved the average from 383,523,500 ns to
263,642,694 ns, about 31.3 percent.

The profiled Q4_K/Q6_K roles shrink substantially. In the default-pool run,
profiled benchmark-token work dropped from 175,834,213 ns to 61,006,374 ns.
End-to-end benchmark time improved less because the timed decode includes work
outside the profiled matvec aggregation.

This is still not a Tier 1 throughput pass. The best Qwen2.5-1.5B run in this
slice was about 4.41 tok/s, below the 10 tok/s Tier 1 target. The path is also
not default eligible until the SmolLM2-1.7B parity failures are understood or
the activation quantization strategy is tightened enough to restore parity.
