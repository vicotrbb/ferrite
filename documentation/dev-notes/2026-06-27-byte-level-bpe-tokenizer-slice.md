# 2026-06-27 Byte-Level BPE Tokenizer Slice

## Scope

This slice updates Ferrite's GGUF BPE tokenizer path to seed merge candidates
from UTF-8 bytes mapped through the GPT-2 byte-to-Unicode alphabet.

## Implementation

- Changed BPE seeding from `char` values to encoded bytes.
- Added the GPT-2 byte alphabet mapping used by GGUF GPT-2-style tokenizers.
- Updated the synthetic BPE fixture to use `Ġ` for byte-level spaces.
- Added coverage for spaces and non-ASCII UTF-8 bytes before BPE merges.

## Boundaries

This improves real SmolLM2 text prompt parity, but it does not complete chat
template rendering, special-token parsing controls, or every model-specific
pre-tokenizer variant.

## Evidence

- Real-model red before this slice:
  - `llama-tokenize` encoded `hello world` as `[28120, 905]`.
  - Ferrite failed `--prompt 'hello world'` with `no BPE seed token matches " "`.
  - `llama-tokenize` encoded `café` as `[83, 1939, 2756]`.
  - Ferrite previously encoded `café` as `83,1939,180`.
- Test red: `cargo test -p ferrite-model --test tokenizer_metadata
  bpe_seeds_from_gpt2_byte_alphabet_before_merging` failed on a literal-space
  seed.
- Test green: `cargo test -p ferrite-model --test tokenizer_metadata` passed
  with byte-level BPE coverage.
- Real-model tokenizer green:
  - `hello world`: llama.cpp `[28120, 905]`, Ferrite `prompt_token_ids=28120,905`
  - ` hello`: llama.cpp `[33662]`, Ferrite `prompt_token_ids=33662`
  - `café`: llama.cpp `[83, 1939, 2756]`, Ferrite `prompt_token_ids=83,1939,2756`
- Real-model next-token comparison:
  - Ferrite `--prompt 'hello world'` returned `next_token_id=30`, `next_token=.`
  - `llama-simple -n 1 'hello world'` generated `hello world.`
  - `llama-tokenize --prompt '.'` returned `[30]`
