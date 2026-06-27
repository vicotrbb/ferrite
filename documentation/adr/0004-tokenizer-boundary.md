# ADR 0004: GGUF Tokenizer Metadata Boundary

Date: 2026-06-27

Status: Accepted

## Context

Ferrite's first milestone requires a deterministic next-token path for a tiny
Llama-family GGUF model. The scalar executor can now consume generated
unquantized tensors, but model prompts and reference comparisons need a
tokenizer boundary. GGUF files can carry tokenizer metadata such as
`tokenizer.ggml.model`, `tokenizer.ggml.tokens`, and
`tokenizer.ggml.token_type`.

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

This is not yet a full Llama BPE tokenizer. It is a small verified boundary
that lets Ferrite test tokenizer metadata extraction and simple prompt fixtures
without pretending merge rules are complete.

## Consequences

The tokenizer module stays separate from `gguf.rs`, keeping the binary parser
focused. Future BPE merge handling can be added inside the tokenizer module or
submodules without growing the GGUF parser into an unrelated responsibility.

The GGUF parser now accepts metadata-only or tokenizer-only files with zero
tensors. This is required for tokenizer fixtures and GGUF vocab sidecars.

## Alternatives Considered

Implement full BPE immediately.

This was rejected for this slice because BPE merge behavior needs dedicated
fixtures and reference comparisons. The atomic tokenizer still provides useful
metadata extraction and deterministic decode coverage.

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
