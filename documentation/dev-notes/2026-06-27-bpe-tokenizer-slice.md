# 2026-06-27 BPE Tokenizer Slice

## Scope

This slice extends the GGUF tokenizer boundary with fixture-grade ranked BPE
merge encoding from `tokenizer.ggml.merges`.

## Implementation

- `GgufTokenizer` now stores optional GGUF merge metadata.
- `GgufTokenizer::encode_bpe` delegates to `tokenizer::bpe`.
- `tokenizer::bpe` seeds symbols from existing one-character tokens, applies
  merge rules in GGUF rank order, and maps the final symbols back to token IDs.

## Boundaries

This is not full Llama tokenizer parity. Remaining tokenizer work includes
normalizer behavior, pre-tokenization, byte fallback tokens, special-token
policy, chat templates, and comparison against a real reference tokenizer.

## Evidence

- Red: `cargo test -p ferrite-model --test tokenizer_metadata
  encodes_with_ranked_bpe_merges_from_gguf_metadata` failed because
  `GgufTokenizer` had no `encode_bpe` method.
- Green: the same targeted test passed after adding the focused BPE helper.
