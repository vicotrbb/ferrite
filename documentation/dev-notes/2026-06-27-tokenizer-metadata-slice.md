# 2026-06-27 Tokenizer Metadata Slice

## Slice

Add a focused tokenizer boundary over GGUF metadata. This moves Ferrite closer
to real Tier 0 reference comparison by making tokenizer metadata visible to
Rust code and by supporting deterministic atomic-token fixtures.

## Implementation

- Added `ferrite_model::tokenizer`.
- Added `GgufTokenizer::from_gguf`.
- Added tokenizer model detection for `tokenizer.ggml.model`.
- Added token and token-type extraction from:
  - `tokenizer.ggml.tokens`
  - `tokenizer.ggml.token_type`
- Added token decode from token ID sequences.
- Added greedy longest-prefix `encode_atomic` for deterministic simple
  fixtures.
- Updated the GGUF parser to allow metadata-only files with zero tensors.

## Validation

TDD red step:

```text
cargo test -p ferrite-model --test tokenizer_metadata
error[E0432]: unresolved import `ferrite_model::tokenizer`
```

Parser bug exposed by the tokenizer fixture:

```text
cargo test -p ferrite-model --test tokenizer_metadata
Error: GgufError { message: "tensor data start is past end of file" }
```

Green step:

```text
cargo test -p ferrite-model --test tokenizer_metadata
test result: ok. 2 passed; 0 failed
```

## Remaining Unproven

- Full Llama BPE merge behavior is not implemented.
- Tokenizer normalizer/pre-tokenizer behavior is not implemented.
- The tokenizer has not been compared against a real Tier 0 tokenizer artifact.
- Prompt formatting and chat templates are not implemented.

## Next Slice

Implement BPE merge handling from GGUF `tokenizer.ggml.merges` fixtures, then
compare against a known reference tokenizer for a tiny Llama-family model.
