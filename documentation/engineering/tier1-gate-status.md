# Tier 1 Gate Status

Date: 2026-06-28

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
Q4_K matvec paths checked against scalar oracles. It also has focused x86_64
AVX2 runtime backend evidence for F32, Q8_0, Q5_0, Q4_K, and Q6_K matvec paths
on a bounded homelab amd64 pod.

Ferrite now has several real Tier 1 model-output proofs: SmolLM2-1.7B-Instruct
Q4_K_M matched fixed local `llama.cpp` deterministic reference profiles for
six prompts, and Qwen2.5-0.5B-Instruct Q4_K_M plus Qwen2.5-1.5B-Instruct
Q4_K_M matched fixed local `llama.cpp` deterministic reference profiles for
six prompts. Qwen2.5-0.5B-Instruct Q8_0 and Q6_K also matched fixed local
`llama.cpp` deterministic reference profiles for the same six prompts locally
and on x86_64 AVX2. Qwen2.5-1.5B-Instruct Q8_0 and Q6_K now also match those
six prompt profiles locally and on x86_64 AVX2; x86_64 throughput is now
measured and remains below target.
The Qwen2.5-1.5B proof exercises the Tier 1 head_dim=128 model.
The Q4_K and Q6_K SIMD paths also have row-level Rayon parallelism on aarch64
NEON and compile-checked x86_64 AVX2, and the Q4_K, Q5_0, and Q6_K aarch64
paths now use fused NEON block-dot helpers.
Local SmolLM2-1.7B benchmark improvements are recorded, and `--benchmark-runs`
now uses a token-id-only repeated-token path instead of returning unused logits
for each benchmark token. Generated-token loops also use token-id-only repeated
acceptance after the first prompt next-token result, and CLI generation now
stops after emitting tokenizer EOS when GGUF metadata provides
`tokenizer.ggml.eos_token_id`. A next-token profiling CLI identifies
per-operation matvec timings for real Tier 1 models and includes storage kind,
shape, and storage bytes for each profiled matrix. It also emits aggregate
role/signature summaries for profile-driven optimization. Benchmark token
profiling now records token-id decode profiles from a replay session outside
the timed benchmark loop. Tier 1 does not yet prove broad x86_64 AVX2
model-output parity, x86_64 throughput, broad 0.5B-1.7B prompt/model coverage,
or full-tier throughput.
Qwen2.5-0.5B Q4_K_M now has local default-pool and two-thread cached-token
benchmarks above 10 tok/s after the Q8_0 direct argmax and thresholded Q5_0
row-parallel slices. The opt-in Q4_K/Q6_K x Q8_K activation matvec path improves
local Qwen2.5-1.5B benchmark-token throughput, but the best observed run remains
below 10 tok/s and the path is not default eligible because SmolLM2-1.7B parity
still fails. Local Qwen2.5-1.5B Q8_0 and Q6_K benchmark runs measured about
6.48 tok/s and 3.69 tok/s on the default pool, also below target.

## Evidence Matrix

| Criterion | Status | Evidence |
| --- | --- | --- |
| GQA variants 1:1, 3:1, 4:1, 6:1, 7:1 | Proven for scalar attention harness | `documentation/dev-notes/2026-06-27-tier1-gqa-ratio-harness.md`; `cargo test -p ferrite-inference gqa_broadcasts_kv_heads_for_tier1_ratios -- --nocapture` |
| RoPE `head_dim=64` and `head_dim=128` | Proven for scalar RoPE harness | `documentation/dev-notes/2026-06-27-tier1-rope-head-dim-harness.md`; `cargo test -p ferrite-inference rope_rotates_full_tier1_head_dimensions -- --nocapture` |
| KV cache grows and shrinks across turns | Proven for scalar session cache | `documentation/dev-notes/2026-06-27-tier1-session-cache-truncation.md`; `cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture` |
| Matvec kernels compare against scalar reference within explicit tolerance | Harness covers F32, Q8_0, Q5_0, Q4_K, and Q6_K public matrix paths | `documentation/dev-notes/2026-06-27-tier1-matvec-kernel-check.md`; `cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture` |
| AArch64 SIMD correctness | Partially proven for F32, Q8_0, Q5_0, Q6_K, and Q4_K matvec on local NEON host | `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-f32-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q5-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q6-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q4-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-q4k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-27-tier1-q6k-row-parallel-simd.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q5-neon-block-dot.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; targeted aarch64 backend tests |
| AVX2 correctness | Focused runtime backend tests passed for F32, Q8_0, Q5_0, Q6_K, and Q4_K on x86_64 AVX2 homelab pods; the fixed six-prompt Tier 1 artifact set now includes Qwen2.5-1.5B Q8_0/Q6_K matching x86_64 AVX2 model-output evidence; throughput evidence is tracked separately in the Tier 1 throughput row | `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-f32-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q5-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q6-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-x86-64-avx2-q4-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-q4k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-27-tier1-q6k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-runtime-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-prompt-expansion.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-smollm2-1-7b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-fifth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-prompt-closure.md`; `cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests`; `cargo test -p ferrite-inference avx2 -- --nocapture`; `cargo test -p ferrite-inference simd_matvec_preserves_parallel_row_order -- --nocapture`; `cargo test -p ferrite-inference q8_0_argmax_mul_vec_matches_full_matvec_argmax -- --nocapture`; x86_64 fixed SmolLM2 prompts matched `[18,198,3725,198,198,788]`, `[7042,30,2]`, `[28,281,253,1165,6560,32047]`, `[338,2433,253,1837,3500,1743]`, `[597,325,804,288,6524,260]`, and `[216,34,12382,282,7367,30]`; x86_64 fixed Qwen2.5-0.5B Q4_K_M prompts matched `[198,9707,11]`, `[12095,13,1084]`, `[11,1052,572]`, `[429,374,6188]`, `[387,1483,311,7023,279,28636]`, and `[220,18,25374,315,19828,13]`; x86_64 fixed Qwen2.5-1.5B prompts matched `[198,9707,11]`, `[12095,13,576]`, `[11,1052,572]`, `[429,374,6188]`, `[387,1483,311,7023,279,28636]`, and `[220,17,25374,315,19828,323]`; x86_64 Qwen2.5-0.5B Q8_0 and Q6_K matched `[198,9707,11,4337,0,2585]`, `[12095,13,1084,374,279,7772]`, `[11,1052,572,264,3908,883]`, `[429,374,6188,311,387,6092]`, `[387,1483,311,7023,279,28636]`, and `[220,18,25374,315,19828,13]`; x86_64 Qwen2.5-1.5B Q8_0 and Q6_K matched `[198,9707,11,1879,0,2585]`, `[12095,13,576,6722,315,9625]`, `[11,1052,572,264,3908,3743]`, `[429,374,6188,311,387,6092]`, `[387,1483,311,7023,279,3853]`, and `[220,17,25374,315,19828,323]` |
| Quantized SIMD correctness | Partially proven for Q8_0, Q5_0, Q6_K, and Q4_K on local NEON host | `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q8-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q5-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q6-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-aarch64-neon-q4-matvec.md`; `documentation/dev-notes/2026-06-27-tier1-q4k-row-parallel-simd.md`; `documentation/dev-notes/2026-06-27-tier1-q6k-row-parallel-simd.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q5-neon-block-dot.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; Q4_K and Q6_K dispatch is scoped to rows whose column count is a whole number of K-blocks |
| Real 0.5B-1.7B model output | Partially proven for six fixed 1.7B Llama-family prompts and six fixed Qwen2 prompts; new Qwen2.5-1.5B Q8_0/Q6_K evidence now also includes x86_64 AVX2 | `documentation/dev-notes/2026-06-27-tier1-smollm2-1-7b-reference-probe.md`; `documentation/dev-notes/2026-06-28-tier1-smollm2-second-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-third-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-fourth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-fifth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-sixth-prompt-reference.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-rope-layout.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-1-5b-reference-probe.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-second-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q8-0-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q6-k-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-1-5b-q8-q6-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-prompt-expansion.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-smollm2-1-7b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-fifth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-prompt-closure.md`; Ferrite matched local `llama.cpp` token IDs `[18, 198, 3725, 198, 198, 788]`, `[7042,30,2]`, `[28,281,253,1165,6560,32047]`, `[338,2433,253,1837,3500,1743]`, `[597,325,804,288,6524,260]`, and `[216,34,12382,282,7367,30]` for SmolLM2-1.7B-Instruct Q4_K_M; both Qwen2.5 Q4_K_M models matched their documented first-six prompt continuations locally and on x86_64 AVX2; Qwen2.5-0.5B-Instruct Q8_0 and Q6_K matched the same six Qwen2.5-0.5B prompt profiles locally and on x86_64 AVX2; Qwen2.5-1.5B-Instruct Q8_0 and Q6_K matched six local and x86_64 AVX2 prompt profiles with token IDs `[198,9707,11,1879,0,2585]`, `[12095,13,576,6722,315,9625]`, `[11,1052,572,264,3908,3743]`, `[429,374,6188,311,387,6092]`, `[387,1483,311,7023,279,3853]`, and `[220,17,25374,315,19828,323]` |
| Qwen2 Tier 1 model coverage | Partially proven for Qwen2.5-0.5B and 1.5B Q4_K_M over six fixed prompts locally and on x86_64 AVX2, plus Qwen2.5-0.5B Q8_0 and Q6_K over the same six prompts locally and on x86_64 AVX2, and Qwen2.5-1.5B Q8_0 and Q6_K locally and on x86_64 AVX2 | `documentation/dev-notes/2026-06-27-tier1-qwen2-0-5b-probe.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-config-parser.md`; `documentation/dev-notes/2026-06-27-scalar-qkv-projection-bias.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-loader-dispatch.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-rope-layout.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-1-5b-reference-probe.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-second-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-third-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-fourth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-fifth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-sixth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q8-0-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q6-k-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-1-5b-q8-q6-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-fifth-prompt-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-prompt-closure.md`; Qwen2.5-0.5B-Instruct and Qwen2.5-1.5B-Instruct Q4_K_M both load, use split-half RoPE, and match deterministic local `llama.cpp` reference continuations for the six fixed prompts locally and on x86_64 AVX2; Qwen2.5-0.5B-Instruct Q8_0 and Q6_K also load and match those six prompt profiles locally and on x86_64 AVX2; Qwen2.5-1.5B-Instruct Q8_0 and Q6_K load and match those six prompt profiles locally and on x86_64 AVX2 |
| Tier 1 throughput target | Partially proven for local Qwen2.5-0.5B Q4_K_M only; x86_64 Tier 1 pod runs remained below target; not proven for the full tier | `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-scalar-probe.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-row-parallel.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-row-parallel.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-token-id-benchmark.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-q4k-throughput.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q5-neon-block-dot.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q8-argmax.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-0-5b-q5-thresholded-row-parallel.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-current-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-k-opt-in-benchmark.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-throughput.md`; `documentation/benchmarks/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-throughput.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q8-0-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q6-k-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-prompt-expansion.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-smollm2-1-7b-model-check.md`; token-id benchmark path improved the SmolLM2-1.7B local default-pool run to about 5.51 tok/s and the 2-thread run to about 3.36 tok/s; Qwen2.5-0.5B improved to about 15.55 tok/s default-pool and 12.29 tok/s with `RAYON_NUM_THREADS=2` after thresholded Q5_0 row-parallel scheduling on local aarch64; bounded local Qwen2.5-0.5B Q8_0 and Q6_K runs measured about 52,794,900 ns and 52,152,800 ns per benchmark token for `hello world`, with maximum resident set sizes 830,095,360 and 1,024,950,272 bytes; bounded x86_64 AVX2 Qwen2.5-0.5B Q8_0 and Q6_K runs measured about 71,197,566 ns and 96,172,191 ns per benchmark token for `hello world` on the default pool, and about 73,319,573 ns and 102,300,612 ns with `RAYON_NUM_THREADS=2`; the bounded x86_64 AVX2 Qwen2.5-0.5B Q4_K_M pod runs ranged from about 2.71 to 3.17 tok/s default-pool and about 2.78 to 3.16 tok/s with `RAYON_NUM_THREADS=2`; the bounded x86_64 AVX2 Qwen2.5-1.5B pod run was about 0.56 tok/s default-pool and 0.56 tok/s with `RAYON_NUM_THREADS=2`; the bounded x86_64 AVX2 SmolLM2-1.7B pod run was about 0.65 tok/s default-pool and 0.65 tok/s with `RAYON_NUM_THREADS=2`; opt-in Q8_K improved the local aarch64 Qwen2.5-1.5B Q4_K_M default-pool run from 261,316,083 ns, about 3.83 tok/s, to 226,673,736 ns, about 4.41 tok/s, and the two-thread run from 383,523,500 ns, about 2.61 tok/s, to 263,642,694 ns, about 3.79 tok/s; local Qwen2.5-1.5B Q8_0 measured 154,366,350 ns, about 6.48 tok/s default-pool and 156,033,066 ns, about 6.41 tok/s with `RAYON_NUM_THREADS=2`; local Qwen2.5-1.5B Q6_K measured 271,304,008 ns, about 3.69 tok/s default-pool and 451,274,383 ns, about 2.22 tok/s with `RAYON_NUM_THREADS=2`; bounded x86_64 AVX2 Qwen2.5-1.5B Q8_0 measured 234,260,802 ns, about 4.27 tok/s default-pool and 229,229,300 ns, about 4.36 tok/s with `RAYON_NUM_THREADS=2`; bounded x86_64 AVX2 Qwen2.5-1.5B Q6_K measured 1,356,563,424 ns, about 0.74 tok/s default-pool and 1,348,577,646 ns, about 0.74 tok/s with `RAYON_NUM_THREADS=2`; broader Tier 1 throughput remains below or unproven |
| Generated token path | Proven for token-id-only repeated acceptance and EOS stopping on real 1.7B profiles | `documentation/dev-notes/2026-06-27-token-id-generation-path.md`; `documentation/dev-notes/2026-06-28-cli-eos-generation-stop.md`; generated-token loops use token-id-only repeated acceptance and still matched SmolLM2-1.7B token IDs `[18, 198, 3725, 198, 198, 788]`; CLI generation stops after emitting tokenizer EOS when GGUF metadata provides `tokenizer.ggml.eos_token_id`, proven by the SmolLM2-1.7B `The capital of France is` check with `[7042,30,2]` |
| Next-token and benchmark-token operation profiling | Proven for CLI, one real 1.7B next-token profile, and Qwen2 next-token/current benchmark-token profiles | `documentation/dev-notes/2026-06-27-tier1-next-token-profile.md`; `documentation/dev-notes/2026-06-27-tier1-profile-matrix-metadata.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-benchmark-token-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-current-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-profile.md`; `--profile-next-token` emits per-operation labels, matrix storage kind/shape/bytes, and aggregate `profile_next_token_role` summaries; `--profile-benchmark-token` profiles the first token-id benchmark decode outside the timed benchmark loop; SmolLM2-1.7B points at Q4_K/Q6_K FFN/output roles, Qwen2.5-0.5B points at Q5_0 FFN gate/up, the Qwen2.5-1.5B Q4_K_M profile still points at Q4_K FFN plus Q6_K output/down roles, and the Qwen2.5-1.5B Q8_0/Q6_K profile shows Q8_0 faster overall while Q6_K is slower in FFN and projection roles but faster in the final output projection |
| Rejected optimization experiments | Q8_0 and Q5_0 naive row-level Rayon scheduling regressed and were reverted; Q6_K argmax `try_reduce` row reduction and Q4_K/Q6_K thresholded row-parallel scheduling were tested and not retained | `documentation/dev-notes/2026-06-27-tier1-q8-row-parallel-regression.md`; `documentation/dev-notes/2026-06-27-tier1-q5-row-parallel-regression.md`; `documentation/dev-notes/2026-06-27-tier1-q6-argmax-reduction-regression.md`; `documentation/dev-notes/2026-06-28-tier1-q4-q6-thresholded-row-parallel-regression.md`; Q8_0 was implemented in `3b12756` and reverted in `1ae4275`; Q5_0 was implemented in `f318e3b` and reverted in `a5d9382`; Q6_K argmax row reduction regressed Qwen2.5-1.5B from `295,683,141` ns to `302,361,766` ns default-pool and from `378,677,558` ns to `593,748,308` ns with `RAYON_NUM_THREADS=2`; Q4_K/Q6_K thresholded row-parallel scheduling regressed the fresh Qwen2.5-1.5B default-pool run from `278,971,500` ns to `357,896,986` ns |
| Next Q4_K/Q6_K kernel hypothesis | Implemented as an explicit experimental parity-scoped policy, not eligible for default dispatch | `documentation/research/2026-06-28-tier1-q4-q6-kernel-hypothesis.md`; `documentation/dev-notes/2026-06-28-q8-k-opt-in-dispatch.md`; `documentation/dev-notes/2026-06-28-q8-k-q6-argmax-options.md`; `documentation/dev-notes/2026-06-28-q8-k-reference-arithmetic.md`; `documentation/dev-notes/2026-06-28-q8-k-smollm-boundary-probes.md`; `documentation/dev-notes/2026-06-28-q8-k-activation-policy.md`; `documentation/benchmarks/2026-06-28-tier1-q8-k-activation-dot.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-k-opt-in-benchmark.md`; Ferrite now has an experimental Q4_K/Q6_K x Q8_K activation matvec route behind `--experimental-q8-k-activation-matvec` and reports `q8_k_activation_matvec_policy=experimental_parity_scoped` when enabled; token-id-only Q6_K output argmax now honors the experimental option; the reference-arithmetic audit and SmolLM boundary probes found no localized Q4_K/Q6_K x Q8_K formula hole against llama.cpp contracts, and the Qwen2.5-1.5B benchmark improved with the flag, but SmolLM2-1.7B diverged on both fixed prompts while the default path still matched, so the Q8_K route must remain opt-in and cannot replace default dispatch |

## Fresh Full-Workspace Gate

Commands run after the x86_64 AVX2 fixed-prompt closure evidence slice:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

All commands passed on commit `ebff88c`.

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

- Expand real model-output coverage beyond the fixed six-prompt profiles and
  current Q4_K_M plus Qwen2.5-0.5B and local Qwen2.5-1.5B
  Q8_0/Q6_K quantization set.
- Expand Tier 1 model coverage beyond the six fixed SmolLM2-1.7B-Instruct
  Q4_K_M local reference profiles recorded so far.
- Expand Qwen2 coverage beyond the fixed Q4_K_M, Qwen2.5-0.5B Q8_0/Q6_K, and
  Qwen2.5-1.5B Q8_0/Q6_K prompt profiles. Broader prompt coverage, broader
  x86_64 throughput beyond the current benchmark set, and full-tier throughput
  remain unproven.
- Continue optimizing hot matvec formats and decode scheduling; Qwen2.5-0.5B
  now has local default-pool and two-thread runs above 10 tok/s after Q5_0
  fused block-dot plus Q8_0 direct argmax work, but Qwen2.5-1.5B and SmolLM2
  1.7B remain below the Tier 1 throughput target.
- Use `--profile-next-token` to isolate hot operation labels and matrix
  metadata before the next optimization slice. The latest SmolLM2-1.7B profile
  still points at the large Q6_K output projection after the Q4_K/Q6_K
  fused-dot improvements, while Qwen2.5-0.5B points at Q5_0 FFN gate/up and
  Qwen2.5-1.5B points at Q4_K FFN plus Q6_K output/down roles.
- Treat the Q4_K/Q6_K x Q8_K path as an approved experimental parity-scoped
  kernel-contract path, not a default dispatch path. The reference arithmetic
  audit found no formula hole, the SmolLM boundary probes did not find a
  localized output-projection or Q6_K-only hole, and the Qwen2.5-1.5B benchmark
  shows real throughput value. SmolLM2-1.7B multi-token parity still fails from
  accumulated activation drift, so any promotion still requires broader parity
  evidence or a tighter activation quantization strategy.
- Do not reapply naive Q8_0 or Q5_0 row-level Rayon scheduling without first
  isolating hot tensors and testing a threshold or fused strategy; direct copies
  of the Q4_K/Q6_K pattern regressed and were reverted.
- Do not replace the Q6_K argmax row-score collection with the tested Rayon
  `try_reduce` shape; it regressed Qwen2.5-1.5B, especially with
  `RAYON_NUM_THREADS=2`.
- Benchmark optimized Tier 1 decode throughput with hardware, model,
  quantization, prompt, thread count, and RSS details before making any
  throughput claim.
- Keep Tier 0's SmolLM2-360M CPU-only reference split documented as a caveat
  when comparing optimized CPU paths.
