# 2026-06-28 Tier 1 Qwen2 0.5B Q5_0 Thresholded Row Parallel

## Scope

This benchmark records the incremental effect of adding thresholded aarch64
NEON row-parallel scheduling for Q5_0 matrices with many rows and moderate
width.

This is deliberately not the reverted naive Q5_0 row-parallel shape. The new
path is gated to `rows >= 4096` and `cols <= 1024`, matching the Qwen2.5-0.5B
Q5_0 FFN gate/up tensors identified by `--profile-benchmark-token`.

This is local Tier 1 optimization evidence for Qwen2.5-0.5B only. It does not
prove full Tier 1 throughput.

## Tree State

- Branch: `main`
- Implementation commit: this atomic Q5_0 thresholded row-parallel commit
- Working tree before slice: clean after `7e489ba`

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- Physical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 397,808,192 bytes
- Scalar weight bytes reported by Ferrite: 391,859,712 bytes

## TDD and Verification

Red test before implementation:

```text
error[E0599]: no variant, associated function, or constant named `Aarch64NeonRowParallel` found for enum `Q5_0MatVecBackend` in the current scope
```

Focused green tests:

```sh
cargo test -p ferrite-inference q5_0_large_moderate_width_matvec_uses_neon_row_parallel -- --nocapture
cargo test -p ferrite-inference q5_0 -- --nocapture
```

Additional verification:

```sh
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
cargo build --release -p ferrite-cli
```

All commands passed.

## Commands

Parity check:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
```

Default Rayon-pool benchmark and benchmark-token profile:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
```

Two-thread benchmark and benchmark-token profile:

```sh
/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 3 --profile-benchmark-token
```

## Results

| Run | benchmark_avg_ns | Approx tok/s | profile_total_ns | Real time | User time | Max RSS | Peak footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Before thresholded Q5_0 row parallel, default pool | 88,910,986 | 11.25 | 87,019,882 | 1.37 s | 1.05 s | 799,637,504 bytes | 827,935,808 bytes |
| Thresholded Q5_0 row parallel, default pool | 64,301,166 | 15.55 | 48,537,753 | 1.14 s | 1.10 s | 779,419,648 bytes | 827,804,608 bytes |
| Before thresholded Q5_0 row parallel, `RAYON_NUM_THREADS=2` | 92,898,566 | 10.76 | not measured | 1.14 s | 0.84 s | 685,539,328 bytes | 827,771,840 bytes |
| Thresholded Q5_0 row parallel, `RAYON_NUM_THREADS=2` | 81,336,111 | 12.29 | 75,022,373 | 1.38 s | 1.00 s | 602,292,224 bytes | 828,361,856 bytes |

The default-pool before row is from
`documentation/benchmarks/2026-06-28-tier1-qwen2-benchmark-token-profile.md`.
The two-thread before row is from
`documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q8-argmax.md`.

## Evidence

Parity check:

```text
generated_token_ids=198,9707,11
generated_match=true
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=122880
```

Default Rayon-pool benchmark:

```text
benchmark_runs=3
benchmark_cached_tokens=5
benchmark_token_ids=9707,11,1879
benchmark_total_ns=192903500
benchmark_avg_ns=64301166
profile_benchmark_token_input_id=198
profile_benchmark_token_id=9707
profile_benchmark_token_total_ns=48537753
profile_benchmark_token_role=ffn_down:Q4_K:896:4864:2451456:3547958
profile_benchmark_token_role=ffn_down:Q6_K:896:4864:3575040:4110836
profile_benchmark_token_role=ffn_gate:Q5_0:4864:896:2996224:8202249
profile_benchmark_token_role=ffn_up:Q5_0:4864:896:2996224:7172460
profile_benchmark_token_role=output:Q8_0:151936:896:144643072:14144875
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=122880
        1.14 real         1.10 user         0.44 sys
           779419648  maximum resident set size
           827804608  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=3
benchmark_cached_tokens=5
benchmark_token_ids=9707,11,1879
benchmark_total_ns=244008333
benchmark_avg_ns=81336111
profile_benchmark_token_input_id=198
profile_benchmark_token_id=9707
profile_benchmark_token_total_ns=75022373
profile_benchmark_token_role=ffn_down:Q4_K:896:4864:2451456:7765333
profile_benchmark_token_role=ffn_down:Q6_K:896:4864:3575040:9174002
profile_benchmark_token_role=ffn_gate:Q5_0:4864:896:2996224:16008960
profile_benchmark_token_role=ffn_up:Q5_0:4864:896:2996224:15992914
profile_benchmark_token_role=output:Q8_0:151936:896:144643072:14600875
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=122880
        1.38 real         1.00 user         0.37 sys
           602292224  maximum resident set size
           828361856  peak memory footprint
```

## Interpretation

The thresholded Q5_0 row-parallel path improved the local Qwen2.5-0.5B default
pool benchmark from 88,910,986 ns to 64,301,166 ns per repeated token. The
profiled Q5_0 FFN gate/up roles dropped from 52,540,750 ns to 15,374,709 ns.

The two-thread benchmark also stayed above the Tier 1 target in this local run,
with 81,336,111 ns per repeated token. This avoids the known failed Q5_0
experiment's broad row-parallel scheduling by limiting the path to the
large-row, moderate-width profile that was measured as hot in Qwen2.5-0.5B.

Ferrite still needs broader prompts, larger Tier 1 model throughput, and AVX2
runtime evidence before Tier 1 throughput can be marked complete.
