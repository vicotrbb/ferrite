# 2026-06-27 Text Prompt Bridge Slice

## Scope

This slice connects GGUF tokenizer metadata to the scalar inference prompt path.

## Implementation

- Added `GgufTokenizer::encode`, choosing BPE encoding when merge metadata is
  present and atomic longest-prefix encoding otherwise.
- Added `ScalarLlamaModel::next_token_for_text_prompt`.
- Added `crates/ferrite-inference/src/scalar/prompt.rs` so text prompt encoding
  stays separate from scalar forward execution and GGUF loading.
- Extended the scalar GGUF fixture with `tokenizer.ggml.model` metadata so the
  fixture can instantiate `GgufTokenizer`.

## Boundaries

This does not add a CLI, real Tier 0 model artifact, reference-runtime
comparison, or full tokenizer parity. It proves the repository has a tested
bridge from text prompt to token IDs to scalar next-token inference.

## Evidence

- Red: `cargo test -p ferrite-inference --test scalar_reference
  text_prompt_path_encodes_with_gguf_tokenizer_before_forward` failed because
  `ScalarLlamaModel` had no `next_token_for_text_prompt` method.
- Green: the same targeted test passed after adding the prompt bridge and
  tokenizer encoding convenience.
