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
below 10 tok/s and the path is not default eligible because fixed-prompt parity
still fails for SmolLM2-1.7B and Qwen2.5-1.5B under tested experimental
policies. Local Qwen2.5-1.5B Q6_K benchmark runs measured about 3.69 tok/s on
the default pool, below target. Local Qwen2.5-1.5B Q8_0 now exceeds the 10 tok/s
single-model benchmark-token target after the thresholded parallel output
argmax route and the gated Q8_0 NEON row-parallel FFN gate/up route: the
current benchmark-token profile measures 82,289,555 ns, about 12.15 tok/s.
Local Qwen2.5-1.5B Q8_0 and Q6_K memory posture is now bounded for short
one-token CLI probes with both peak RSS and post-load current RSS samples. The
same artifacts also have bounded CLI KV-cache growth evidence through 65 cached
tokens. The local Qwen2.5-1.5B Q8_0 and Q6_K OpenAI-compatible server paths
also have bounded post-load and sequential one-token request RSS samples. The
full Tier 1 throughput gate remains open because broader models, quantizations,
prompts, x86_64 throughput, full-tier memory posture, and HTTP throughput are
not yet proven.

The OpenAI-compatible HTTP server now has opt-in real Tier 1 coverage for
Qwen2.5-0.5B Q4_K_M through legacy completions, streaming legacy completions,
chat completions, and streaming chat completions, plus explicit Qwen2.5-1.5B
Q8_0 legacy completion, streaming legacy completion, and chat completion
proofs, including streaming chat. The Qwen2.5-1.5B Q6_K server path also has
bounded one-token latency coverage for the same four endpoint shapes. This
proves the local server path can drive real Tier 1 GGUF models for deterministic
one-token responses, and bounded local latency benchmarks now measure the
Qwen2.5-0.5B and Qwen2.5-1.5B Q8_0/Q6_K one-token HTTP paths. It does not prove
Tier 1 server throughput, concurrent real-model successful serving, or broader
Tier 1 server behavior. A separate opt-in proof now verifies real Tier 1
backpressure:
a concurrent request receives an OpenAI-shaped `429 rate_limit_error` while a
longer streaming request holds the single inference permit. The generic OpenAI
server path now also supports an operator-configured bounded wait window before
returning backpressure, proven with fixture HTTP coverage, a real
Qwen2.5-0.5B Q4_K_M overlap proof, and real Qwen2.5-1.5B Q8_0 overlap,
Q8_0/Q6_K three-request queue-order, and Q8_0/Q6_K six-prompt
legacy-completion proofs while preserving the single-inference-permit runtime
invariant.

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
| Tier 1 memory posture | Bounded locally for one-token Qwen2.5-1.5B Q8_0 and Q6_K CLI probes with peak RSS and post-load current RSS samples; bounded locally for Qwen2.5-1.5B Q8_0 and Q6_K CLI KV-cache growth through 65 cached tokens; bounded locally for Qwen2.5-1.5B Q8_0 and Q6_K OpenAI-compatible server post-load RSS and three sequential one-token endpoint cycles; full-tier memory posture remains unproven | `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-memory.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-kv-growth.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-server-memory.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q6-server-memory.md`; `documentation/dev-notes/2026-06-28-cli-load-memory-sampling-pause.md`; Qwen2.5-1.5B Q8_0 reported `model_file_bytes=1894532128`, `scalar_weight_bytes=1888581632`, `kv_cache_bytes=172032`, max RSS 3,821,076,480 bytes, peak footprint 3,822,526,784 bytes, and post-load CLI current RSS 1,926,742,016 bytes; Qwen2.5-1.5B Q6_K reported `model_file_bytes=1464178720`, `scalar_weight_bytes=1458228224`, `kv_cache_bytes=172032`, max RSS 2,961,850,368 bytes, peak footprint 2,962,856,576 bytes, and post-load CLI current RSS 1,497,677,824 bytes; both CLI one-token memory runs reported `model_file_retained_bytes=0` and matched expected generated token ID `198`; Q8_0 and Q6_K KV-cache growth probes both reported 172,032 bytes at 3 cached tokens, 974,848 bytes at 17 cached tokens, and 3,727,360 bytes at 65 cached tokens, exactly 57,344 bytes per cached token; the Qwen2.5-1.5B Q8_0 server repeated pass sampled 1,928,036,352 bytes after health and 1,958,854,656 bytes after three sequential cycles covering legacy completion, chat completion, legacy completion streaming, and chat streaming with HTTP `200` responses; the Qwen2.5-1.5B Q6_K server repeated pass sampled 1,498,955,776 bytes after health and 1,544,093,696 bytes after the same three sequential endpoint cycles with HTTP `200` responses |
| OpenAI-compatible Tier 1 HTTP path | Proven for deterministic one-token Qwen2.5-0.5B Q4_K_M local server completions and chat, both non-streaming and SSE streaming; proven for deterministic one-token Qwen2.5-1.5B Q8_0 and Q6_K legacy completion and chat, both non-streaming and SSE streaming; bounded single-client Qwen2.5-0.5B and Qwen2.5-1.5B Q8_0/Q6_K latency measured; bounded real Tier 1 backpressure proven for a second concurrent request; configured bounded wait proven on fixture HTTP, one real Qwen2.5-0.5B Q4_K_M overlap path, and real Qwen2.5-1.5B Q8_0 overlap plus Q8_0/Q6_K three-request queue-order and six-prompt legacy-completion paths; broader concurrent serving throughput remains unproven | `documentation/dev-notes/2026-06-28-openai-real-tier1-http-proof.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-http-streaming-proof.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-http-chat-proof.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-http-backpressure-proof.md`; `documentation/dev-notes/2026-06-28-openai-inference-wait-timeout.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-http-bounded-wait-proof.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-qwen-1-5b-q8-http-proof.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-qwen-1-5b-q8-bounded-wait-proof.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-qwen-1-5b-q8-queue-regression.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-qwen-1-5b-q6-queue-regression.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-qwen-1-5b-q8-prompt-regression.md`; `documentation/dev-notes/2026-06-28-openai-real-tier1-qwen-1-5b-q6-prompt-regression.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-0-5b-http-latency.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-http-latency.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q6-http-latency.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-queue-order.md`; `cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture`; `cargo test -p ferrite-server --test openai_real_tier1_http live_http_server_waits_for_concurrent_real_tier1_request -- --ignored --nocapture`; `cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http -- --ignored --nocapture`; `cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_queue live_http_server_serves_qwen_1_5b_q8_wait_queue_in_start_order -- --ignored --nocapture`; `cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_queue live_http_server_serves_qwen_1_5b_q6_wait_queue_in_start_order -- --ignored --nocapture`; `cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts -- --ignored --nocapture`; `cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_matches_qwen_1_5b_q6_first_tokens_for_reference_prompts -- --ignored --nocapture`; `cargo test -p ferrite-server -- --nocapture`; explicit Qwen2.5-1.5B Q8_0 HTTP run passed `live_http_server_generates_with_qwen_1_5b_q8_model`, `live_http_server_streams_with_qwen_1_5b_q8_model`, `live_http_server_chats_with_qwen_1_5b_q8_model`, and `live_http_server_streams_chat_with_qwen_1_5b_q8_model` in 121.30s; explicit Qwen2.5-1.5B Q8_0 bounded-wait overlap run passed `live_http_server_waits_for_concurrent_qwen_1_5b_q8_request` in 108.79s after the 16-token long-stream shape correctly exhausted the 180s wait window with `429`; a bounded local Qwen2.5-1.5B Q8_0 queue-order probe launched one 4-token streaming chat holder followed by two queued one-token completions; all three returned HTTP `200`, the stream emitted `[DONE]`, and the queued completions finished in request-start order; the matching ignored regression `live_http_server_serves_qwen_1_5b_q8_wait_queue_in_start_order` passed in 163.88s; the matching Q6_K queue regression `live_http_server_serves_qwen_1_5b_q6_wait_queue_in_start_order` passed in 160.93s; the six-prompt Qwen2.5-1.5B Q8_0 legacy-completion regression `live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts` passed in 296.39s; the matching Q6_K regression `live_http_server_matches_qwen_1_5b_q6_first_tokens_for_reference_prompts` passed in 286.58s; explicit Qwen2.5-0.5B real Tier 1 run passed 5 ignored real Tier 1 HTTP tests, including `live_http_server_rejects_concurrent_real_tier1_request`; targeted bounded-wait real Tier 1 run passed `live_http_server_waits_for_concurrent_real_tier1_request` in 110.22s; fixture package verification passed 50 unit tests, 7 `async-openai` client integration tests, and 6 fixture live HTTP integration tests; bounded local HTTP averages for one-token Qwen2.5-0.5B Q4_K_M were about 240.944 ms for legacy completion, 573.722 ms for chat, 251.947 ms for legacy streaming, and 642.878 ms for chat streaming; bounded local HTTP averages for one-token Qwen2.5-1.5B Q8_0 were about 309.739 ms for legacy completion, 805.396 ms for chat, 316.647 ms for legacy streaming, and 819.067 ms for chat streaming; bounded local HTTP averages for one-token Qwen2.5-1.5B Q6_K were about 821.195 ms for legacy completion, 2370.354 ms for chat, 828.764 ms for legacy streaming, and 2327.475 ms for chat streaming |
| Tier 1 throughput target | Partially proven for local Qwen2.5-0.5B Q4_K_M and local Qwen2.5-1.5B Q8_0; x86_64 Tier 1 pod runs remained below target; not proven for the full tier | `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-scalar-probe.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-row-parallel.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-row-parallel.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q4k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-q6k-fused-neon.md`; `documentation/benchmarks/2026-06-27-tier1-smollm2-1-7b-token-id-benchmark.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-q4k-throughput.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q5-neon-block-dot.md`; `documentation/benchmarks/2026-06-27-tier1-qwen2-0-5b-q8-argmax.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-0-5b-q5-thresholded-row-parallel.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-current-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-current-head-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-parallel-argmax.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-gated-row-parallel.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-k-opt-in-benchmark.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-throughput.md`; `documentation/benchmarks/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-throughput.md`; `documentation/dev-notes/2026-06-28-q6-k-avx2-argmax.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q8-0-reference.md`; `documentation/dev-notes/2026-06-28-tier1-qwen2-0-5b-q6-k-reference.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-prompt-expansion.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-0-5b-q8-q6-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-qwen2-1-5b-model-check.md`; `documentation/dev-notes/2026-06-28-tier1-avx2-smollm2-1-7b-model-check.md`; token-id benchmark path improved the SmolLM2-1.7B local default-pool run to about 5.51 tok/s and the 2-thread run to about 3.36 tok/s; Qwen2.5-0.5B improved to about 15.55 tok/s default-pool and 12.29 tok/s with `RAYON_NUM_THREADS=2` after thresholded Q5_0 row-parallel scheduling on local aarch64; bounded local Qwen2.5-0.5B Q8_0 and Q6_K runs measured about 52,794,900 ns and 52,152,800 ns per benchmark token for `hello world`, with maximum resident set sizes 830,095,360 and 1,024,950,272 bytes; bounded x86_64 AVX2 Qwen2.5-0.5B Q8_0 and Q6_K runs measured about 71,197,566 ns and 96,172,191 ns per benchmark token for `hello world` on the default pool, and about 73,319,573 ns and 102,300,612 ns with `RAYON_NUM_THREADS=2`; the bounded x86_64 AVX2 Qwen2.5-0.5B Q4_K_M pod runs ranged from about 2.71 to 3.17 tok/s default-pool and about 2.78 to 3.16 tok/s with `RAYON_NUM_THREADS=2`; the bounded x86_64 AVX2 Qwen2.5-1.5B pod run was about 0.56 tok/s default-pool and 0.56 tok/s with `RAYON_NUM_THREADS=2`; the bounded x86_64 AVX2 SmolLM2-1.7B pod run was about 0.65 tok/s default-pool and 0.65 tok/s with `RAYON_NUM_THREADS=2`; opt-in Q8_K improved the local aarch64 Qwen2.5-1.5B Q4_K_M default-pool run from 261,316,083 ns, about 3.83 tok/s, to 226,673,736 ns, about 4.41 tok/s, and the two-thread run from 383,523,500 ns, about 2.61 tok/s, to 263,642,694 ns, about 3.79 tok/s; local Qwen2.5-1.5B Q8_0 previously measured 154,366,350 ns, about 6.48 tok/s default-pool and 156,033,066 ns, about 6.41 tok/s with `RAYON_NUM_THREADS=2`; after the parallel output argmax route it measured 137,502,638 ns, about 7.27 tok/s; after gated Q8_0 NEON row parallelism for FFN gate/up it measured 82,289,555 ns, about 12.15 tok/s, with FFN down now the largest retained role at 37,516,790 ns; local Qwen2.5-1.5B Q6_K measured 271,304,008 ns, about 3.69 tok/s default-pool and 451,274,383 ns, about 2.22 tok/s with `RAYON_NUM_THREADS=2`; bounded x86_64 AVX2 Qwen2.5-1.5B Q8_0 measured 234,260,802 ns, about 4.27 tok/s default-pool and 229,229,300 ns, about 4.36 tok/s with `RAYON_NUM_THREADS=2`; bounded x86_64 AVX2 Qwen2.5-1.5B Q6_K initially measured 1,356,563,424 ns, about 0.74 tok/s default-pool and 1,348,577,646 ns, about 0.74 tok/s with `RAYON_NUM_THREADS=2`; after the Q6_K AVX2 argmax route it measured 1,086,706,040 ns, about 0.92 tok/s default-pool and 1,111,777,214 ns, about 0.90 tok/s with `RAYON_NUM_THREADS=2`; broader Tier 1 throughput remains below or unproven |
| Generated token path | Proven for token-id-only repeated acceptance and EOS stopping on real 1.7B profiles | `documentation/dev-notes/2026-06-27-token-id-generation-path.md`; `documentation/dev-notes/2026-06-28-cli-eos-generation-stop.md`; generated-token loops use token-id-only repeated acceptance and still matched SmolLM2-1.7B token IDs `[18, 198, 3725, 198, 198, 788]`; CLI generation stops after emitting tokenizer EOS when GGUF metadata provides `tokenizer.ggml.eos_token_id`, proven by the SmolLM2-1.7B `The capital of France is` check with `[7042,30,2]` |
| Next-token and benchmark-token operation profiling | Proven for CLI, one real 1.7B next-token profile, local Qwen2 next-token/current benchmark-token profiles, and x86_64 Qwen2.5-1.5B Q8_0/Q6_K benchmark-token profiles | `documentation/dev-notes/2026-06-27-tier1-next-token-profile.md`; `documentation/dev-notes/2026-06-27-tier1-profile-matrix-metadata.md`; `documentation/dev-notes/2026-06-27-tier1-qwen2-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-benchmark-token-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-current-profile.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-q6-profile.md`; `documentation/benchmarks/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-profile.md`; `--profile-next-token` emits per-operation labels, matrix storage kind/shape/bytes, and aggregate `profile_next_token_role` summaries; `--profile-benchmark-token` profiles the first token-id benchmark decode outside the timed benchmark loop; SmolLM2-1.7B points at Q4_K/Q6_K FFN/output roles, Qwen2.5-0.5B points at Q5_0 FFN gate/up, the local Qwen2.5-1.5B Q4_K_M profile still points at Q4_K FFN plus Q6_K output/down roles, local Qwen2.5-1.5B Q8_0/Q6_K shows Q8_0 faster overall while Q6_K is slower in FFN and projection roles but faster in the final output projection, and post-argmax-route x86_64 Qwen2.5-1.5B Q6_K shows the output role reduced to about 82.6 ms with FFN roles now dominating |
| Rejected optimization experiments | Q8_0 and Q5_0 naive row-level Rayon scheduling regressed and were reverted; Q8_0 whole-row NEON accumulation regressed and was not retained; Q6_K argmax `try_reduce` row reduction and Q4_K/Q6_K thresholded row-parallel scheduling were tested and not retained | `documentation/dev-notes/2026-06-27-tier1-q8-row-parallel-regression.md`; `documentation/dev-notes/2026-06-27-tier1-q5-row-parallel-regression.md`; `documentation/dev-notes/2026-06-28-tier1-q8-0-row-dot-regression.md`; `documentation/dev-notes/2026-06-27-tier1-q6-argmax-reduction-regression.md`; `documentation/dev-notes/2026-06-28-tier1-q4-q6-thresholded-row-parallel-regression.md`; Q8_0 row parallelism was implemented in `3b12756` and reverted in `1ae4275`; Q8_0 whole-row NEON accumulation preserved Qwen2.5-1.5B Q8_0 six-token parity but regressed the current-head benchmark-token profile from `155,274,902` ns to `462,229,625` ns and was reverted before commit; Q5_0 was implemented in `f318e3b` and reverted in `a5d9382`; Q6_K argmax row reduction regressed Qwen2.5-1.5B from `295,683,141` ns to `302,361,766` ns default-pool and from `378,677,558` ns to `593,748,308` ns with `RAYON_NUM_THREADS=2`; Q4_K/Q6_K thresholded row-parallel scheduling regressed the fresh Qwen2.5-1.5B default-pool run from `278,971,500` ns to `357,896,986` ns |
| Next Q4_K/Q6_K kernel hypothesis | Implemented as an explicit experimental parity-scoped policy, not eligible for default dispatch | `documentation/research/2026-06-28-tier1-q4-q6-kernel-hypothesis.md`; `documentation/dev-notes/2026-06-28-q8-k-opt-in-dispatch.md`; `documentation/dev-notes/2026-06-28-q8-k-q6-argmax-options.md`; `documentation/dev-notes/2026-06-28-q8-k-reference-arithmetic.md`; `documentation/dev-notes/2026-06-28-q8-k-smollm-boundary-probes.md`; `documentation/dev-notes/2026-06-28-q8-k-activation-policy.md`; `documentation/dev-notes/2026-06-28-q8-k-role-scoped-policy.md`; `documentation/dev-notes/2026-06-28-q8-k-role-scope-probes.md`; `documentation/dev-notes/2026-06-28-q8-k-single-role-probes.md`; `documentation/dev-notes/2026-06-28-q8-k-ffn-up-six-prompt-probe.md`; `documentation/dev-notes/2026-06-28-q8-k-qwen-1-5b-parity-probe.md`; `documentation/dev-notes/2026-06-28-q8-k-qwen-1-5b-divergence-profile.md`; `documentation/benchmarks/2026-06-28-tier1-q8-k-activation-dot.md`; `documentation/benchmarks/2026-06-28-tier1-qwen2-1-5b-q8-k-opt-in-benchmark.md`; Ferrite now has an experimental Q4_K/Q6_K x Q8_K activation matvec route behind `--experimental-q8-k-activation-matvec` and reports `q8_k_activation_matvec_policy=experimental_parity_scoped` when enabled; token-id-only Q6_K output argmax now honors the experimental option; the reference-arithmetic audit found no localized Q4_K/Q6_K x Q8_K formula hole, but SmolLM2-1.7B and Qwen2.5-1.5B fixed-prompt probes both found parity failures under tested experimental policies, so the Q8_K route must remain opt-in and cannot replace default dispatch; the Qwen divergence profile shows another narrow top-logit margin inverted by activation drift |

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
- Expand memory evidence beyond the bounded local one-token Qwen2.5-1.5B
  Q8_0/Q6_K CLI probes, 65-token KV-cache growth probes, and Qwen2.5-1.5B
  Q8_0/Q6_K sequential server RSS probes. Full-tier memory posture,
  large-context KV-cache growth, concurrent server memory behavior, and
  longer-running steady-state RSS remain unproven.
- Expand real Tier 1 HTTP server coverage beyond deterministic one-token
  Qwen2.5-0.5B Q4_K_M completions/chat, deterministic one-token Qwen2.5-1.5B
  Q8_0/Q6_K completions/chat, and the bounded single-client latency and
  backpressure runs. The configured bounded-wait path now has one real
  Qwen2.5-0.5B Q4_K_M overlap proof plus real Qwen2.5-1.5B Q8_0 overlap,
  Q8_0/Q6_K three-request queue-order probes, and Q8_0/Q6_K six-prompt
  legacy-completion probes, but server throughput, broader successful
  concurrent real-model serving, broader queue fairness, long-stream overlap,
  and broader model/prompt behavior remain unproven.
- Continue optimizing hot matvec formats and decode scheduling; Qwen2.5-0.5B
  now has local default-pool and two-thread runs above 10 tok/s after Q5_0
  fused block-dot plus Q8_0 direct argmax work, but Qwen2.5-1.5B and SmolLM2
  1.7B remain below the Tier 1 throughput target.
- Use `--profile-next-token` or `--profile-benchmark-token` to isolate hot
  operation labels and matrix metadata before the next optimization slice. The
  latest SmolLM2-1.7B profile still points at the large Q6_K output projection
  after the Q4_K/Q6_K fused-dot improvements, Qwen2.5-0.5B points at Q5_0 FFN
  gate/up, current local Qwen2.5-1.5B Q8_0 points primarily at Q8_0 FFN down
  after the large-row parallel argmax route reduced the output role and gated
  row parallelism reduced FFN gate/up, local Qwen2.5-1.5B Q4_K_M points at
  Q4_K FFN plus Q6_K output/down roles, and x86_64 Qwen2.5-1.5B Q6_K now points to
  transformer-layer Q6_K FFN roles after the output projection was improved
  with the AVX2 argmax route.
- Treat the Q4_K/Q6_K x Q8_K path as an approved experimental parity-scoped
  kernel-contract path, not a default dispatch path. The reference arithmetic
  audit found no formula hole, the SmolLM boundary probes did not find a
  localized output-projection or Q6_K-only hole, and the Qwen2.5-1.5B benchmark
  shows real throughput value. SmolLM2-1.7B multi-token parity still fails from
  accumulated activation drift, so any promotion still requires broader parity
  evidence or a tighter activation quantization strategy. Use the role-scoped
  experimental policy to isolate candidate safe role subsets before any broader
  promotion discussion; the first broad-scope probes did not find a default-safe
  role subset, and the follow-up six-prompt `ffn_up` probe rejects the remaining
  single-role default-dispatch candidate for SmolLM2-1.7B. The all-role
  Qwen2.5-1.5B probe also diverged on one fixed prompt, so benchmark improvement
  must remain separated from parity approval.
- Do not reapply naive Q8_0 or Q5_0 row-level Rayon scheduling without first
  isolating hot tensors and testing a threshold or fused strategy; direct copies
  of the Q4_K/Q6_K pattern regressed and were reverted. Do not reapply the
  tested Q8_0 whole-row NEON accumulation shape either; it preserved parity but
  regressed Qwen2.5-1.5B Q8_0 benchmark-token throughput.
- Do not replace the Q6_K argmax row-score collection with the tested Rayon
  `try_reduce` shape; it regressed Qwen2.5-1.5B, especially with
  `RAYON_NUM_THREADS=2`.
- Benchmark optimized Tier 1 decode throughput with hardware, model,
  quantization, prompt, thread count, and RSS details before making any
  throughput claim.
- Keep Tier 0's SmolLM2-360M CPU-only reference split documented as a caveat
  when comparing optimized CPU paths.
