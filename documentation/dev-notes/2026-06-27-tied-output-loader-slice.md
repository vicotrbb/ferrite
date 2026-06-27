# 2026-06-27 Tied Output Loader Slice

## Scope

This slice adds scalar loader support for GGUF models that omit
`output.weight` and reuse `token_embd.weight` as the output projection.

## Implementation

- Added `scalar_llama_tied_output_f32_gguf_fixture`.
- Added a scalar loader test that omits `output.weight`.
- Updated the scalar GGUF loader to clone `token_embd.weight` when
  `output.weight` is absent and shape-compatible.

## Boundaries

This is a compatibility fallback for tied embeddings. It does not infer more
complex output heads or architecture-specific projection variants.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  falls_back_to_token_embeddings_for_tied_output_weight` failed because
  `output.weight` was required.
- Green: the same targeted test passed after adding the tied-output fallback.
