# ADR 0004: GGUF Tokenizer Metadata Boundary

Date: 2026-06-27

Status: Accepted

## Context

Ferrite's first milestone requires a deterministic next-token path for a tiny
Llama-family GGUF model. The scalar executor can now consume generated
unquantized tensors, but model prompts and reference comparisons need a
tokenizer boundary. GGUF files can carry tokenizer metadata such as
`tokenizer.ggml.model`, `tokenizer.ggml.tokens`, and
`tokenizer.ggml.token_type`. Some GGUF vocabularies also carry ranked BPE
merge rules under `tokenizer.ggml.merges`.

## Decision

Ferrite adds `ferrite_model::tokenizer` as a focused module separate from the
GGUF binary parser. The first tokenizer boundary extracts GGUF tokenizer
metadata and provides deterministic operations for simple atomic-token
fixtures:

- tokenizer model identification.
- token ID to text lookup.
- token type lookup.
- token ID sequence decoding.
- greedy longest-prefix atomic encoding.
- fixture-grade ranked BPE merge encoding from `tokenizer.ggml.merges`.

This is not yet a full Llama tokenizer. It is a small verified boundary that
lets Ferrite test tokenizer metadata extraction, simple prompt fixtures, and
ranked merge metadata without pretending normalizer, pre-tokenizer, byte
fallback, or chat template behavior is complete.

## Consequences

The tokenizer module stays separate from `gguf.rs`, keeping the binary parser
focused. BPE merge handling lives below the tokenizer module in a focused
submodule instead of growing the public tokenizer boundary into a broad parser.

The GGUF parser now accepts metadata-only or tokenizer-only files with zero
tensors. This is required for tokenizer fixtures and GGUF vocab sidecars.

## Alternatives Considered

Implement full tokenizer parity immediately.

This was rejected for this slice because full parity needs normalizer,
pre-tokenizer, byte fallback, and reference comparisons against a real tokenizer
implementation. The current BPE path still provides useful metadata extraction
and deterministic merge coverage.

Store tokenizer helpers in `gguf.rs`.

This was rejected to avoid turning the GGUF parser into a broad model/tokenizer
utility module.

## Evidence

- `cargo test -p ferrite-model --test tokenizer_metadata` first failed because
  `ferrite_model::tokenizer` did not exist.
- After adding the tokenizer module, the same test exposed a GGUF parser bug:
  metadata-only GGUF fixtures failed with `tensor data start is past end of
  file`.
- After allowing zero-tensor GGUF files, `cargo test -p ferrite-model --test
  tokenizer_metadata` passed 2 tests covering metadata extraction, decode, and
  greedy atomic encoding.
- `cargo test -p ferrite-model --test tokenizer_metadata
  encodes_with_ranked_bpe_merges_from_gguf_metadata` first failed because
  `GgufTokenizer` had no `encode_bpe` method.
- After adding the focused BPE helper, the same targeted test passed with GGUF
  `tokenizer.ggml.merges` driving ranked merges for a tokenizer fixture.
