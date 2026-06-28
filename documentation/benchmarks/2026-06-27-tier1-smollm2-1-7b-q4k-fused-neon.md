# 2026-06-27 Tier 1 SmolLM2 1.7B Q4_K Fused NEON Dot

## Scope

This benchmark records the incremental effect of replacing the aarch64 Q4_K
NEON path's decode-then-dot block loop with a fused block-dot helper. The helper
decodes each Q4_K block directly into NEON multiply-accumulate lanes instead of
materializing a full `[f32; 256]` block before the dot product.

This is still not a Tier 1 throughput pass. The local 2-thread run remains
below the `>= 10 tok/s` target.

## Tree State

- Branch: `main`
- Implementation commit: `065685b`
- Working tree before note: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/SmolLM2-1.7B-Instruct-GGUF`
- File: `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 1,055,609,824 bytes
- Scalar weight bytes reported by Ferrite: 1,053,827,072 bytes
- Quantization: Q4_K_M GGUF mixture

## TDD and Verification

Red test before implementation:

```text
error[E0432]: unresolved import `super::neon_q4_k_block_dot`
```

Focused green test:

```sh
cargo test -p ferrite-inference q4_k -- --nocapture
```

Full verification before the implementation commit:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
git diff --check
```

All commands passed.

## Commands

Release build:

```sh
cargo build --release -p ferrite-cli
```

Parity check:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788
```

Default Rayon-pool benchmark:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Two-thread benchmark:

```sh
RAYON_NUM_THREADS=2 /usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Profile summary:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --profile-next-token
```

## Results

| Run | benchmark_avg_ns | Approx Tok/s | Real Time | User Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Q4_K + Q6_K row parallel, default Rayon pool | 317,917,433 | 3.145 | 3.81 s | 7.35 s | 1,475,821,568 bytes | 2,123,830,464 bytes |
| Q4_K fused NEON dot, default Rayon pool | 229,523,316 | 4.357 | 3.81 s | 4.47 s | 1,522,860,032 bytes | 2,123,994,432 bytes |
| Q4_K + Q6_K row parallel, `RAYON_NUM_THREADS=2` | 549,736,508 | 1.819 | 4.73 s | 6.46 s | 1,828,929,536 bytes | 2,123,551,872 bytes |
| Q4_K fused NEON dot, `RAYON_NUM_THREADS=2` | 339,890,275 | 2.942 | 3.88 s | 3.85 s | 1,475,592,192 bytes | 2,123,633,856 bytes |

The prior Q4_K + Q6_K row-parallel rows are from
`documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-row-parallel.md`.

## Full Output

Parity check:

````text
prompt_token_ids=28120,905
next_token_id=18
next_token="
generated_cached_tokens=8
generated_token_ids=18,198,3725,198,198,788
generated_text="
```

In
expected_generated_token_ids=18,198,3725,198,198,788
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=3145728
expected_token_id=18
match=true
````

Default Rayon-pool benchmark:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_total_ns=1147616584
benchmark_avg_ns=229523316
model_file_bytes=1055609824
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
        3.81 real         4.47 user         2.17 sys
          1522860032  maximum resident set size
          2123994432  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=5
benchmark_cached_tokens=7
benchmark_total_ns=1699451375
benchmark_avg_ns=339890275
model_file_bytes=1055609824
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
        3.88 real         3.85 user         1.01 sys
          1475592192  maximum resident set size
          2123633856  peak memory footprint
```

Profile summary after the change:

```text
profile_next_token_total_ns=118711240
profile_next_token_role=ffn_gate:Q4_K:8192:2048:9437184:21893872
profile_next_token_role=ffn_up:Q4_K:8192:2048:9437184:21898998
profile_next_token_role=ffn_down:Q4_K:2048:8192:9437184:10320790
profile_next_token_role=q_proj:Q4_K:2048:2048:2359296:7951333
profile_next_token_role=output:Q6_K:49152:2048:82575360:9858875
```

## Interpretation

The fused Q4_K NEON block dot improved the local default-pool cached-token
average from about 0.318 seconds to about 0.230 seconds. The 2-thread average
improved from about 0.550 seconds to about 0.340 seconds.

The built-in profile summary shows the Q4_K FFN gate/up roles dropping from the
previous roughly 42-45 ms range to about 22 ms each on this run. Q6_K output and
Q6_K `ffn_down` remain visible hot paths.

This is retained Tier 1 optimization evidence, but Ferrite is still below the
Tier 1 throughput target.

