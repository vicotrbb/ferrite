# 2026-06-27 CLI Next-Token Slice

## Scope

This slice adds the first command-line entry point for loading a GGUF file and
running a text prompt through the scalar next-token path.

## Implementation

- Added `crates/ferrite-cli` with binary name `ferrite`.
- Added focused CLI modules for argument parsing and runtime execution.
- The command accepts `--model <path.gguf>` and `--prompt <text>`.
- The command parses GGUF, builds `GgufTokenizer`, loads the unquantized scalar
  Llama model, runs `next_token_for_text_prompt`, and prints:
  - `next_token_id=<id>`
  - `next_token=<token text>`

## Boundaries

This is still fixture-validated bring-up plumbing. It does not claim real Tier 0
model compatibility, llama.cpp parity, quantized tensor support, or performance
readiness.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_loads_gguf_and_prints_text_prompt_next_token` failed because the
  `ferrite` binary target did not exist.
- Green: the same targeted test passed after adding the CLI implementation.
