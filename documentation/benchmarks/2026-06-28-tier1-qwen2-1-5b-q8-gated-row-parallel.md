# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 Gated Row Parallel

## Scope

This note records the local aarch64 benchmark result after adding thresholded
Q8_0 NEON row parallelism for large row count and moderate width matrices.

This is evidence for local Qwen2.5-1.5B Q8_0 throughput. It does not complete
Tier 1 because broader quantization, prompt, model, architecture, memory, and
HTTP throughput coverage remain separate gates.

## Tree State

- Branch: `main`
- Code commit: `af898b5`
- Optimization: `perf: gate q8 neon row parallelism`
- Release CLI rebuilt before profiling:

```sh
cargo build --release -p ferrite-cli
```

## Hypothesis

The earlier naive Q8_0 row-parallel experiment regressed because it scheduled
all multi-row Q8_0 shapes with Rayon. The retained hypothesis is narrower:
parallelize only matrices with at least 4096 rows and at most 2048 columns.
For Qwen2.5-1.5B Q8_0 this targets FFN gate/up matrices shaped
`8960x1536` while leaving the wide `1536x8960` FFN down matrix serial.

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

The new dispatch shape was added with a focused red/green backend test:

```sh
cargo test -p ferrite-inference q8_0_large_moderate_width_matvec_uses_neon_row_parallel -- --nocapture
```

The first red run failed because `Q8_0MatVecBackend::Aarch64NeonRowParallel`
did not exist. After implementation, both the new row-parallel test and the
small-shape serial NEON backend test passed.

The broader local inference checks also passed:

```sh
cargo test -p ferrite-inference q8_0 -- --nocapture
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
| 82,289,555 | 12.15 | 76,735,136 | 1.31 s | 1.60 s | 0.54 s | 3,821,027,328 bytes | 3,822,477,632 bytes |

Token output:

```text
benchmark_runs=3
benchmark_cached_tokens=5
benchmark_token_ids=9707,11,1879
benchmark_total_ns=246868666
benchmark_avg_ns=82289555
```

Aggregate profile roles:

```text
profile_benchmark_token_role=ffn_down:Q8_0:1536:8960:14622720:37516790
profile_benchmark_token_role=ffn_gate:Q8_0:8960:1536:14622720:10183167
profile_benchmark_token_role=ffn_up:Q8_0:8960:1536:14622720:9566584
profile_benchmark_token_role=k_proj:Q8_0:256:1536:417792:1065254
profile_benchmark_token_role=o_proj:Q8_0:1536:1536:2506752:6377421
profile_benchmark_token_role=output:Q8_0:151936:1536:247959552:4538459
profile_benchmark_token_role=q_proj:Q8_0:1536:1536:2506752:6412835
profile_benchmark_token_role=v_proj:Q8_0:256:1536:417792:1074626
```

## Comparison

The previous current-head profile after parallel output argmax measured:

```text
benchmark_avg_ns=137502638
profile_benchmark_token_total_ns=130456877
profile_benchmark_token_role=ffn_gate:Q8_0:8960:1536:14622720:36709918
profile_benchmark_token_role=ffn_up:Q8_0:8960:1536:14622720:36801793
profile_benchmark_token_role=ffn_down:Q8_0:1536:8960:14622720:36779543
```

This slice reduced the benchmark-token average from 137,502,638 ns to
82,289,555 ns, improving the local Qwen2.5-1.5B Q8_0 path from about
7.27 tok/s to about 12.15 tok/s. Compared with the pre-argmax baseline
of 155,274,902 ns, the local path improved from about 6.44 tok/s to
about 12.15 tok/s.

The profile confirms the intended shape selectivity: Q8_0 FFN gate/up dropped
to about 10.18 ms and 9.57 ms, while FFN down remains around 37.52 ms and is
now the largest retained role.

Memory remains high and should be evaluated separately before making memory
efficiency claims. This run reported max RSS 3,821,027,328 bytes and peak
footprint 3,822,477,632 bytes.
