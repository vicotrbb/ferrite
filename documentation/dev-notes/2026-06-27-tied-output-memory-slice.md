# 2026-06-27 Tied Output Memory Slice

## Scope

This slice removes the scalar loader's F32 clone of `token_embd.weight` when a
GGUF model omits `output.weight` and therefore uses tied output embeddings.

## Implementation

- Added `ScalarLlamaOutputWeights` in `crates/ferrite-inference/src/scalar/output.rs`.
- Represented untied output weights as an owned matrix.
- Represented tied output weights as `TiedTokenEmbedding`.
- Resolved the logits matrix at session execution time by borrowing
  `token_embedding` for tied-output models.
- Updated scalar memory accounting so tied output weights add no duplicate
  F32 matrix bytes.

## Boundaries

This does not remove dequantized F32 storage for the embedding itself or for
other tensors. It only avoids an avoidable clone when the output projection is
tied to token embeddings.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  falls_back_to_token_embeddings_for_tied_output_weight` failed with
  `left: 184` and `right: 160`, proving the fixture still counted a duplicate
  output matrix.
- Green: the same targeted test passed after introducing
  `ScalarLlamaOutputWeights`.
- Regression check: `cargo test -p ferrite-inference --test scalar_reference
  scalar_model_reports_weight_and_session_kv_cache_bytes` passed.
- Full verification before commit:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `git diff --check`
  - hygiene scan reported only the existing `Cargo.toml:16:unsafe_code =
    "forbid"` policy line.
