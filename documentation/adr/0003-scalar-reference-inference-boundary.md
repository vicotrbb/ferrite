# ADR 0003: Scalar Reference Inference Boundary

Date: 2026-06-27

Status: Accepted

## Context

Ferrite's first milestone requires a correctness-first forward path before any
SIMD, mmap, quantized kernels, or platform-specific execution path is trusted.
The GGUF reader can expose metadata and tensor ranges, but the project still
needs a small executable inference boundary that can be compared against known
reference values.

## Decision

Ferrite adds a `ferrite-inference` crate for core execution logic. Its first
module is `ferrite_inference::scalar`, a safe Rust scalar reference path.

The scalar boundary includes:

- Row-major dense matrix views for reference fixtures.
- Shape-checked matrix-vector multiply.
- RMSNorm.
- RoPE rotation.
- Multi-token causal Llama-style GQA attention over an in-memory K/V cache.
- Incremental scalar session state that reuses per-layer K/V cache entries
  across accepted tokens.
- SwiGLU feed-forward execution.
- Final logits and deterministic argmax selection.
- GGUF F32/F16/BF16 tensor loading into the scalar Llama weight structure.
- Fixture-validated GGML Q8_0, Q5_0, Q4_K, and Q6_K tensor dequantization into
  the scalar reference path.

The initial path is deliberately synthetic and does not claim real-model
correctness yet. Its purpose is to provide a clear scalar reference target that
later quantization, incremental KV cache logic, tokenizer, and SIMD paths must
match.

## Consequences

Scalar correctness remains the baseline for future optimized work. Any SIMD,
quantized, mmap-backed, or platform-specific path must be tested against this
or a more complete scalar reference path before it can be treated as correct.

The current scalar path includes a tokenizer bridge, mixed GGUF quantization
decoding, real Tier 0 GGUF model loading, and an incremental session cache
boundary. It still does not include optimized quantized matmul kernels, mmap
weight access, threaded execution, or production serving APIs. Those are
follow-up slices.

## Alternatives Considered

Start with optimized quantized kernels.

This was rejected because the project requires scalar reference
implementations before trusting SIMD or quantized paths.

Use an external runtime as the first executor.

This was rejected because Ferrite's inference core must remain Ferrite-owned.
External runtimes remain valid comparison references.

## Evidence

- `cargo test -p ferrite-inference --test scalar_reference` first failed
  because the `scalar` module did not exist.
- After implementing the scalar path, the same test passed 3 tests covering
  RMSNorm, matrix-vector shape validation, and deterministic next-token argmax
  for a one-layer synthetic Llama fixture.
- `cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_f32_gguf_fixture`
  first failed because `ScalarLlamaModel::from_gguf_f32` did not exist.
- After implementing the F32 GGUF loader, the same test passed and produced the
  same deterministic token from generated GGUF tensor bytes.
- `cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_f16_gguf_fixture`
  first failed because the loader rejected `F16` tensors.
- After implementing safe half-precision decoding, the same test passed and
  produced the deterministic scalar token from generated F16 GGUF tensor bytes.
- `cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_bf16_gguf_fixture`
  first failed because the loader rejected `BF16` tensors.
- After implementing safe bfloat16 decoding, the same test passed and produced
  the deterministic scalar token from generated BF16 GGUF tensor bytes.
- `cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_q8_0_gguf_fixture`
  first failed because the loader rejected `Q8_0` tensors.
- After extracting tensor decoding and adding Q8_0 block dequantization, the
  same test passed and produced a deterministic scalar token from generated
  Q8_0 GGUF tensor bytes.
- `cargo test -p ferrite-inference --test scalar_reference loads_scalar_llama_reference_weights_from_q4_k_gguf_fixture`
  first failed because the loader rejected `Q4K` tensors.
- After verifying the upstream Q4_K layout and adding scalar Q4_K block
  dequantization, the same test passed and produced a deterministic scalar
  token from generated mixed F32/Q4_K GGUF tensor bytes.
- `cargo test -p ferrite-inference --test scalar_reference` first failed for
  missing `apply_rope` and `next_token_for_prompt`.
- After implementing RoPE and causal K/V attention, the same test passed 6
  scalar reference tests.
- `cargo test -p ferrite-model --test gguf_reader derives_llama_config_from_uint32_or_uint64_metadata`
  first failed for missing optional RoPE base and RMS epsilon config fields.
- After adding those fields, the same parser test passed.
- `cargo test -p ferrite-inference --test scalar_reference
  scalar_session_reuses_cached_prompt_state_incrementally` first failed because
  `ScalarLlamaModel::start_session` did not exist.
- After adding `ScalarLlamaSession`, the same targeted test passed by matching
  the full-prompt scalar path while retaining cached token count across
  incremental token acceptance.
