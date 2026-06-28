# Tier 1 Gate Status

Date: 2026-06-27

## Scope

Tier 1 covers 0.5B-1.7B models and is intended to prove:

- real matrix sizes;
- SIMD correctness;
- GQA variants;
- RoPE head dimensions used by larger Llama-family models;
- KV-cache growth and shrink behavior; and
- benchmark evidence before throughput claims.

## Current Verdict

Tier 1 is in progress. Ferrite now has several Tier 1 correctness harnesses and
scoped SIMD kernels on the local aarch64 host, but Tier 1 is not
complete.

The current implementation proves scalar GQA ratio coverage, scalar RoPE
coverage for `head_dim=64` and `head_dim=128`, session cache truncation, a
matvec reference-comparison gate, and aarch64 NEON F32, Q8_0, Q5_0, Q6_K, and
Q4_K matvec paths checked against scalar oracles. It also has compile-checked
x86_64 AVX2 dispatch for all currently supported matvec formats, but no x86_64
host runtime evidence yet.

Ferrite now has one real Tier 1 model-output proof: SmolLM2-1.7B-Instruct
Q4_K_M matched a fixed local `llama.cpp` deterministic reference profile for
six generated tokens from the prompt `hello world`. The Q4_K and Q6_K SIMD paths
also have row-level Rayon parallelism on aarch64 NEON and compile-checked x86_64
AVX2, and the Q4_K and Q6_K aarch64 paths now use fused NEON block-dot helpers.
Local SmolLM2-1.7B benchmark improvements are recorded, and `--benchmark-runs`
now uses a token-id-only repeated-token path instead of returning unused logits
for each benchmark token. Generated-token loops also use token-id-only repeated
acceptance after the first prompt next-token result. A next-token profiling CLI
identifies per-operation matvec timings for real Tier 1 models and includes
storage kind, shape, and storage bytes for each profiled matrix. It also emits
aggregate role/signature summaries for profile-driven optimization. Tier 1 does
not yet prove AVX2 runtime correctness, broad 0.5B-1.7B model coverage, Qwen2
architecture support, or throughput.

## Evidence Matrix

| Criterion | Status | Evidence |
| --- | --- | --- |
| GQA variants 1:1, 3:1, 4:1, 6:1, 7:1 | Proven for scalar attention harness | `documentation/dev-notes/2026-06-27-tier1-gqa-ratio-harness.md`; `cargo test -p ferrite-inference gqa_broadcasts_kv_heads_for_tier1_ratios -- --nocapture` |
| RoPE `head_dim=64` and `head_dim=128` | Proven for scalar RoPE harness | `documentation/dev-notes/2026-06-27-tier1-rope-head-dim-harness.md`; `cargo test -p ferrite-inference rope_rotates_full_tier1_head_dimensions -- --nocapture` |
| KV cache grows and shrinks across turns | Proven for scalar session cache | `documentation/dev-notes/2026-06-27-tier1-session-cache-truncation.md`; `cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture` |
| Matvec kernels compare against scalar reference within explicit tolerance | Harness covers F32, Q8_0, Q5_0, Q4_K, and Q6_K public matrix paths | `documentation/dev-notes/2026-06-27-tier1-matvec-kernel-check.md`; `cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture` |
| AArch64 SIMD correctness | Partially proven for F32, Q8_0, Q5_0, Q6_K, and Q4_K matvec on local NEON host | `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-f32-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q5-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q6-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q4-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-q4k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-27-tier1-q6k-row-parallel-simd.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; targeted aarch64 backend tests |
| AVX2 correctness | Compile-only F32, Q8_0, Q5_0, Q6_K, and Q4_K bring-up exists; runtime correctness not proven | `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-f32-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q5-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q6-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q4-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-q4k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-27-tier1-q6k-row-parallel-simd.md`; `cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests`; no x86_64 AVX2 host run yet |
| Quantized SIMD correctness | Partially proven for Q8_0, Q5_0, Q6_K, and Q4_K on local NEON host | `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q5-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q6-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q4-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-q4k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-27-tier1-q6k-row-parallel-simd.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; Q4_K and Q6_K dispatch is scoped to rows whose column count is a whole number of K-blocks |
| Real 0.5B-1.7B model output | Partially proven for one 1.7B model/reference profile | `documentation/dev-notes/2026-06-27-tier1-smollm2-1-7b-reference-probe.md`; Ferrite matched local `llama.cpp` token IDs `[18, 198, 3725, 198, 198, 788]` for SmolLM2-1.7B-Instruct Q4_K_M |
| Qwen2 Tier 1 model coverage | Not supported yet | `documentation/dev-notes/2026-06-27-tier1-qwen2-0-5b-probe.md`; Qwen2.5-0.5B-Instruct Q4_K_M downloaded successfully, but Ferrite exits with `expected llama architecture, found qwen2` |
| Tier 1 throughput target | Not proven | `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-scalar-probe.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-row-parallel.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-row-parallel.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-token-id-benchmark.md`; token-id benchmark path improved the local default-pool run to about 5.51 tok/s and the 2-thread run to about 3.36 tok/s, still below `>= 10 tok/s` |
| Generated token path | Proven for one real 1.7B parity profile | `documentation/dev-notes/2026-06-27-token-id-generation-path.md`; generated-token loops use token-id-only repeated acceptance and still matched SmolLM2-1.7B token IDs `[18, 198, 3725, 198, 198, 788]` |
| Next-token operation profiling | Proven for CLI and one real 1.7B profile | `documentation/dev-notes/2026-06-27-tier1-next-token-profile.md`; `documentation/dev-notes/2026-06-27-tier1-profile-matrix-metadata.md`; `--profile-next-token` emits per-operation labels, matrix storage kind/shape/bytes, and aggregate `profile_next_token_role` summaries; latest SmolLM2-1.7B profile showed Q4_K/Q6_K FFN roles as the largest aggregate matvec group |
| Rejected optimization experiments | Q8_0 and Q5_0 naive row-level Rayon scheduling regressed and were reverted | `documentation/dev-notes/2026-06-27-tier1-q8-row-parallel-regression.md`; `documentation/dev-notes/2026-06-27-tier1-q5-row-parallel-regression.md`; Q8_0 was implemented in `3b12756` and reverted in `1ae4275`; Q5_0 was implemented in `f318e3b` and reverted in `a5d9382` |

## Fresh Full-Workspace Gate

Commands run after the token-id generation path slice:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

All commands passed.

## Fresh Tier 1 Model Probe

Commands run for the SmolLM2-1.7B-Instruct Q4_K_M evidence slice:

```sh
huggingface-cli download bartowski/SmolLM2-1.7B-Instruct-GGUF SmolLM2-1.7B-Instruct-Q4_K_M.gguf --local-dir target/models --max-workers 1
printf 'hello world' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
/usr/bin/time -l target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf -p 'hello world' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --benchmark-runs 5
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --profile-next-token
```

Ferrite's six-token expectation run exited successfully with:

```text
expected_generated_token_ids=18,198,3725,198,198,788
generated_match=true
expected_token_id=18
match=true
```

The benchmark run also exited successfully, but it is scalar baseline evidence,
not a throughput pass:

```text
benchmark_runs=5
benchmark_avg_ns=6333709400
```

After Q4_K row-parallel SIMD, the same benchmark improved substantially but
still did not meet the throughput gate:

```text
benchmark_avg_ns=558353433
```

With `RAYON_NUM_THREADS=2`, it remained below the Tier 1 target:

```text
benchmark_avg_ns=886433241
```

After adding Q6_K row-parallel SIMD as well, the default-pool benchmark
improved again:

```text
benchmark_avg_ns=317917433
```

The corresponding `RAYON_NUM_THREADS=2` run was still below target:

```text
benchmark_avg_ns=549736508
```

After adding profile matrix metadata, the real profile identified exact hot
formats and shapes:

```text
profile_next_token_matrix=layer.0.ffn_gate:Q4_K:8192:2048:9437184
profile_next_token_matrix=layer.0.ffn_up:Q4_K:8192:2048:9437184
profile_next_token_matrix=layer.0.ffn_down:Q6_K:2048:8192:13762560
profile_next_token_matrix=output:Q6_K:49152:2048:82575360
```

The built-in role/signature summary reported:

```text
profile_next_token_role=ffn_down:Q4_K:2048:8192:9437184:20790920
profile_next_token_role=ffn_down:Q6_K:2048:8192:13762560:22756834
profile_next_token_role=ffn_gate:Q4_K:8192:2048:9437184:44502790
profile_next_token_role=ffn_up:Q4_K:8192:2048:9437184:42504875
profile_next_token_role=output:Q6_K:49152:2048:82575360:9603708
```

The same slice kept normal benchmark checks in the retained range, but still
below the Tier 1 target:

```text
benchmark_avg_ns=281364908
RAYON_NUM_THREADS=2 benchmark_avg_ns=559712483
```

After adding the fused Q4_K NEON block-dot path, the benchmark improved again:

```text
benchmark_avg_ns=229523316
RAYON_NUM_THREADS=2 benchmark_avg_ns=339890275
```

The real six-token parity check still matched:

```text
generated_token_ids=18,198,3725,198,198,788
generated_match=true
match=true
```

After adding the fused Q6_K NEON block-dot path, the benchmark improved again:

```text
benchmark_avg_ns=224075783
RAYON_NUM_THREADS=2 benchmark_avg_ns=331853141
```

After changing `--benchmark-runs` to use token-id-only repeated acceptance, the
benchmark improved again:

```text
benchmark_avg_ns=181434575
RAYON_NUM_THREADS=2 benchmark_avg_ns=297401758
```

After changing `--generate-tokens` to use token-id-only repeated acceptance, the
real six-token parity check still matched:

```text
generated_token_ids=18,198,3725,198,198,788
generated_match=true
match=true
```

## Remaining Work

- Run AVX2 runtime correctness checks on an x86_64 host behind ADR 0006's
  unsafe-boundary rules.
- Expand Tier 1 model coverage beyond the single SmolLM2-1.7B-Instruct Q4_K_M
  fixed local reference profile recorded so far.
- Add first-class Qwen2 architecture handling before claiming coverage for the
  Qwen2.5 Tier 1 models. The Qwen2.5-0.5B Q4_K_M probe currently stops at the
  explicit Llama-only loader boundary.
- Continue optimizing hot matvec formats and decode scheduling; Q4_K plus Q6_K
  row parallelism, fused NEON dot paths, and token-id-only repeated acceptance
  are still below the Tier 1 throughput target.
- Use `--profile-next-token` to isolate hot operation labels and matrix
  metadata before the next optimization slice. The latest SmolLM2-1.7B profile
  still points at the large Q6_K output projection after the Q4_K/Q6_K
  fused-dot improvements.
- Do not reapply naive Q8_0 or Q5_0 row-level Rayon scheduling without first
  isolating hot tensors and testing a threshold or fused strategy; direct copies
  of the Q4_K/Q6_K pattern regressed and were reverted.
- Benchmark optimized Tier 1 decode throughput with hardware, model,
  quantization, prompt, thread count, and RSS details before making any
  throughput claim.
- Keep Tier 0's SmolLM2-360M CPU-only reference split documented as a caveat
  when comparing optimized CPU paths.
