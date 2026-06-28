# 2026-06-27 Tier 1 Qwen2 0.5B Q5_0 Fused NEON Dot

## Scope

This benchmark records the incremental effect of replacing the aarch64 Q5_0
NEON path's decode-then-dot block loop with a fused block-dot helper. The helper
decodes one Q5_0 block into four-lane NEON multiply-accumulate batches instead
of materializing a full `[i8; 32]` block before the dot product.

This is still not a Tier 1 throughput pass. The local timed runs remain below
the `>= 10 tok/s` target.

## Tree State

- Branch: `main`
- Implementation commit: this atomic Q5_0 fused-dot commit
- Working tree before slice: clean after `5d6bf4d`

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

## Model

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local size reported by Ferrite: 397,808,192 bytes
- Scalar weight bytes reported by Ferrite: 391,859,712 bytes
- Quantization: Q4_K_M GGUF mixture with Q5_0 FFN gate/up hot paths

## TDD and Verification

Red test before implementation:

```text
error[E0432]: unresolved import `super::neon_q5_0_block_dot`
```

Focused green tests:

```sh
cargo test -p ferrite-inference neon_q5_0_block_dot_matches_decoded_values -- --nocapture
cargo test -p ferrite-inference q5_0 -- --nocapture
```

Additional verification:

```sh
cargo fmt --all -- --check
cargo clippy -p ferrite-inference --all-targets -- -D warnings
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
git diff --check
```

All commands passed.

## Commands

Parity check:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
```

Default Rayon-pool benchmark:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Two-thread benchmark:

```sh
/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
```

Profile command:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --profile-next-token
```

## Results

| Run | benchmark_avg_ns | Approx Tok/s | Real Time | User Time | Max RSS | Peak Footprint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Before Q5_0 fused NEON dot, default Rayon pool | 272,478,316 | 3.67 | 3.66 s | 1.65 s | 827,686,912 bytes | 827,804,928 bytes |
| Q5_0 fused NEON dot, default Rayon pool | 184,524,158 | 5.42 | 2.47 s | 1.53 s | 525,418,496 bytes | 828,115,968 bytes |
| Before Q5_0 fused NEON dot, `RAYON_NUM_THREADS=2` | 250,713,558 | 3.99 | 4.12 s | 1.57 s | 643,104,768 bytes | 828,099,840 bytes |
| Q5_0 fused NEON dot, `RAYON_NUM_THREADS=2` | 190,746,900 | 5.24 | 2.24 s | 1.46 s | 782,401,536 bytes | 828,345,472 bytes |

The before rows are from
`documentation/benchmarks/2026-06-27-tier1-qwen2-q4k-throughput.md`.

## Evidence

Parity check:

```text
generated_token_ids=198,9707,11
generated_match=true
```

Default Rayon-pool benchmark:

```text
benchmark_runs=5
benchmark_total_ns=922620792
benchmark_avg_ns=184524158
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
        2.47 real         1.53 user         0.46 sys
           525418496  maximum resident set size
           828115968  peak memory footprint
```

Two-thread benchmark:

```text
benchmark_runs=5
benchmark_total_ns=953734500
benchmark_avg_ns=190746900
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
        2.24 real         1.46 user         0.24 sys
           782401536  maximum resident set size
           828345472  peak memory footprint
```

Profile summary after the change:

```text
profile_next_token_total_ns=228007204
profile_next_token_role=ffn_gate:Q5_0:4864:896:2996224:74061996
profile_next_token_role=ffn_up:Q5_0:4864:896:2996224:73720042
profile_next_token_role=output:Q8_0:151936:896:144643072:14511208
profile_next_token_role=o_proj:Q5_0:896:896:551936:12693125
profile_next_token_role=q_proj:Q5_0:896:896:551936:13190039
```

## Interpretation

The fused Q5_0 NEON block dot improved the local Qwen2.5-0.5B cached-token
average from about 0.272 seconds to about 0.185 seconds on the default Rayon
pool. The two-thread average improved from about 0.251 seconds to about 0.191
seconds.

The profile command remained noisy on this run and should not be used as a
throughput claim by itself. The timed benchmark rows are the retained evidence
for this slice.

Ferrite remains below the Tier 1 throughput target.
