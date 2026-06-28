# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 Parallel Argmax

## Scope

This note records the local aarch64 benchmark result after adding a
thresholded parallel argmax route for large Q8_0 output-like matrices.

This is evidence for one focused optimization slice. It improves the measured
Qwen2.5-1.5B Q8_0 benchmark-token path, but it does not prove the 10 tok/s
Tier 1 throughput target.

## Tree State

- Branch: `main`
- Code commit: `fe58736`
- Optimization: `perf: parallelize large q8 argmax`
- Release CLI rebuilt before profiling:

```sh
cargo build --release -p ferrite-cli
```

## Model

- File: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- SHA-256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`

Ferrite reported:

```text
model_file_bytes=1894532128
model_file_retained_bytes=0
scalar_weight_bytes=1888581632
kv_cache_bytes=286720
```

## Guardrails

The new helper was added with a focused red/green test:

```sh
cargo test -p ferrite-inference parallel_argmax_q8_0_rows_matches_sequential_argmax -- --nocapture
```

The first red run failed because `parallel_argmax_q8_0_rows` did not exist.
After implementation, the targeted test passed. The broader local inference
checks also passed:

```sh
cargo test -p ferrite-inference -- --nocapture
cargo fmt --all -- --check
cargo clippy -p ferrite-inference --all-targets -- -D warnings
git diff --check
```

Real-model parity for the same prompt still matched the documented token IDs:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --generate-tokens 6 \
  --expect-generated-token-ids 198,9707,11,1879,0,2585
```

```text
generated_token_ids=198,9707,11,1879,0,2585
generated_match=true
```

## Protocol

- Host: local macOS aarch64
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Benchmark runs: 3 repeated token-id decode steps after initial prompt
  next-token computation
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
| 137,502,638 | 7.27 | 130,456,877 | 2.09 s | 1.41 s | 0.33 s | 3,762,143,232 bytes | 3,823,165,760 bytes |

Token output:

```text
benchmark_runs=3
benchmark_cached_tokens=5
benchmark_token_ids=9707,11,1879
benchmark_total_ns=412507916
benchmark_avg_ns=137502638
```

Aggregate profile roles:

```text
profile_benchmark_token_role=ffn_down:Q8_0:1536:8960:14622720:36779543
profile_benchmark_token_role=ffn_gate:Q8_0:8960:1536:14622720:36709918
profile_benchmark_token_role=ffn_up:Q8_0:8960:1536:14622720:36801793
profile_benchmark_token_role=k_proj:Q8_0:256:1536:417792:1056539
profile_benchmark_token_role=o_proj:Q8_0:1536:1536:2506752:6283251
profile_benchmark_token_role=output:Q8_0:151936:1536:247959552:5454000
profile_benchmark_token_role=q_proj:Q8_0:1536:1536:2506752:6306499
profile_benchmark_token_role=v_proj:Q8_0:256:1536:417792:1065334
```

## Comparison

The previous current-head profile at commit `cecf6e4` measured:

```text
benchmark_avg_ns=155274902
profile_benchmark_token_total_ns=155224492
profile_benchmark_token_role=output:Q8_0:151936:1536:247959552:24297958
```

This slice reduced the benchmark-token average from 155,274,902 ns to
137,502,638 ns, improving the local Qwen2.5-1.5B Q8_0 path from about
6.44 tok/s to about 7.27 tok/s. The final output role dropped from
24,297,958 ns to 5,454,000 ns.

The result remains below the 10 tok/s target. The retained hot path now points
primarily at Q8_0 FFN gate/up/down roles.

Memory needs further controlled measurement before any neutral-memory claim:
the reported max RSS increased from 1,671,757,824 bytes in the previous
current-head profile to 3,762,143,232 bytes in this run, while the reported
peak footprint stayed essentially flat at about 3.82 GiB.
