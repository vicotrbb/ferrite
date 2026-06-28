# 2026-06-28 Tier 1 Qwen2 Benchmark Token Profile

## Scope

This benchmark records `--profile-benchmark-token` evidence for the Tier 1
Qwen2 Q4_K_M models after benchmark profiling was moved to a replay session
outside the timed benchmark loop.

This is profile and benchmark evidence only. It does not change runtime code
and does not prove full Tier 1 throughput.

## Tree State

- Branch: `main`
- Commit: `9707d57`
- Working tree before runs: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- Physical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

Commands:

```sh
sysctl -n machdep.cpu.brand_string hw.ncpu hw.physicalcpu hw.logicalcpu hw.memsize
uname -a
```

## Models

### Qwen2.5-0.5B-Instruct Q4_K_M

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 397,808,192 bytes
- Scalar weight bytes reported by Ferrite: 391,859,712 bytes

### Qwen2.5-1.5B-Instruct Q4_K_M

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- File: `qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local size reported by Ferrite: 1,117,320,736 bytes
- Scalar weight bytes reported by Ferrite: 1,111,370,240 bytes

## Protocol

- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Initial next token ID: `198`
- Benchmark runs: 3 repeated token-id decode steps after the initial prompt
  next-token computation
- Profiled benchmark token input: `198`
- Profiled benchmark token output: `9707`
- Thread setting: default Rayon pool
- Memory evidence: `/usr/bin/time -l`

Release build:

```sh
cargo build --release -p ferrite-cli
```

Profile commands:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
```

## Results

| Model | benchmark_avg_ns | Approx tok/s | profile_total_ns | Real time | User time | Max RSS | Peak footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5-0.5B Q4_K_M | 88,910,986 | 11.25 | 87,019,882 | 1.37 s | 1.05 s | 799,637,504 bytes | 827,935,808 bytes |
| Qwen2.5-1.5B Q4_K_M | 288,273,666 | 3.47 | 184,551,073 | 3.99 s | 4.64 s | 2,092,007,424 bytes | 2,268,403,392 bytes |

## Evidence

Qwen2.5-0.5B default pool:

```text
benchmark_runs=3
benchmark_cached_tokens=5
benchmark_token_ids=9707,11,1879
benchmark_total_ns=266732959
benchmark_avg_ns=88910986
profile_benchmark_token_input_id=198
profile_benchmark_token_id=9707
profile_benchmark_token_total_ns=87019882
profile_benchmark_token_role=ffn_down:Q4_K:896:4864:2451456:3878959
profile_benchmark_token_role=ffn_down:Q6_K:896:4864:3575040:5000333
profile_benchmark_token_role=ffn_gate:Q5_0:4864:896:2996224:26519874
profile_benchmark_token_role=ffn_up:Q5_0:4864:896:2996224:26020876
profile_benchmark_token_role=k_proj:Q5_0:128:896:78848:737000
profile_benchmark_token_role=o_proj:Q5_0:896:896:551936:4931045
profile_benchmark_token_role=output:Q8_0:151936:896:144643072:14327875
profile_benchmark_token_role=q_proj:Q5_0:896:896:551936:5103045
profile_benchmark_token_role=v_proj:Q5_0:128:896:78848:344791
profile_benchmark_token_role=v_proj:Q8_0:128:896:121856:156084
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=122880
        1.37 real         1.05 user         0.33 sys
           799637504  maximum resident set size
           827935808  peak memory footprint
```

Qwen2.5-1.5B default pool:

```text
benchmark_runs=3
benchmark_cached_tokens=5
benchmark_token_ids=9707,11,1879
benchmark_total_ns=864821000
benchmark_avg_ns=288273666
profile_benchmark_token_input_id=198
profile_benchmark_token_id=9707
profile_benchmark_token_total_ns=184551073
profile_benchmark_token_role=ffn_down:Q4_K:1536:8960:7741440:19068707
profile_benchmark_token_role=ffn_down:Q6_K:1536:8960:11289600:27429209
profile_benchmark_token_role=ffn_gate:Q4_K:8960:1536:7741440:38375997
profile_benchmark_token_role=ffn_up:Q4_K:8960:1536:7741440:41006331
profile_benchmark_token_role=k_proj:Q4_K:256:1536:221184:12884831
profile_benchmark_token_role=o_proj:Q4_K:1536:1536:1327104:9972042
profile_benchmark_token_role=output:Q6_K:151936:1536:191439360:21634375
profile_benchmark_token_role=q_proj:Q4_K:1536:1536:1327104:10548500
profile_benchmark_token_role=v_proj:Q4_K:256:1536:221184:1990749
profile_benchmark_token_role=v_proj:Q6_K:256:1536:322560:1640332
model_file_bytes=1117320736
scalar_weight_bytes=1111370240
kv_cache_bytes=286720
        3.99 real         4.64 user         2.46 sys
          2092007424  maximum resident set size
          2268403392  peak memory footprint
```

## Interpretation

The replay-profiled benchmark token confirms that Qwen2.5-0.5B remains
Q5_0-heavy in token-id decode. FFN gate/up account for 52,540,750 ns of the
87,019,882 ns profiled token, while the Q8_0 output projection accounts for
14,327,875 ns.

Qwen2.5-1.5B remains dominated by Q4_K FFN gate/up and Q4_K/Q6_K FFN down
work. The Q6_K output projection accounts for 21,634,375 ns, but FFN gate/up
together account for 79,382,328 ns and FFN down accounts for 46,497,916 ns.

The next Qwen2 optimization should stay profile-led. For 0.5B, Q5_0 FFN work is
still the clearest target. For 1.5B, Q4_K/Q6_K FFN work dominates more than the
output projection in this token-id decode profile.
