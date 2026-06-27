# 2026-06-27 CLI Expected-Token Gate Slice

## Scope

This slice adds a deterministic comparison gate to the CLI so model output can
be checked against a documented expected token ID.

## Implementation

- Added optional `--expect-token-id <id>`.
- The CLI prints `expected_token_id=<id>` and `match=<true|false>`.
- A mismatch returns a non-zero process status with an explanatory error.

## Boundaries

This is a local comparison gate. It does not by itself provide llama.cpp parity
or real Tier 0 model evidence; it creates the command surface needed to record
those checks later.

## Evidence

- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_succeeds_when_next_token_matches_expected_id` failed because
  `--expect-token-id` was an unknown argument.
- Red: `cargo test -p ferrite-cli --test next_token_cli
  cli_fails_when_next_token_does_not_match_expected_id` failed while mismatch
  handling still exited successfully.
- Green: `cargo test -p ferrite-cli --test next_token_cli cli_` passed all CLI
  tests after adding the comparison gate.
