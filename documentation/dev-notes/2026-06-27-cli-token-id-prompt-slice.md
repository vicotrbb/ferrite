# 2026-06-27 CLI Token-ID Prompt Slice

## Scope

This slice adds a CLI prompt path that accepts explicit token IDs.

## Implementation

- Added `--prompt-token-ids <id[,id...]>` as an alternative to `--prompt`.
- The CLI prints the token IDs used for inference as `prompt_token_ids=...`.
- The CLI rejects commands that pass both text and token-ID prompt inputs.
- Text prompts still use the GGUF tokenizer path.

## Boundary

This does not implement full tokenizer parity. It creates a stable command
surface for reference comparisons where a known runtime provides the prompt
token IDs and expected next token.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_loads_gguf_and_prints_token_id_prompt_next_token` failed because
  `--prompt-token-ids` was an unknown argument.
- Green: the same targeted test passed after adding prompt source parsing and
  direct token-ID inference.
- `cargo test -p ferrite-cli --test next_token_cli
  cli_rejects_mixed_text_and_token_id_prompts` passed for the mutual-exclusion
  guard.
