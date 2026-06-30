# CLI Next-Token Test Support Split

Date: 2026-06-30

## Scope

This slice splits shared support code out of
`crates/ferrite-cli/tests/next_token_cli.rs` without changing the CLI assertions
or command coverage.

The refactor adds:

- `crates/ferrite-cli/tests/support/fixtures.rs` for fixture model creation,
  binary lookup, and temporary model cleanup.
- `crates/ferrite-cli/tests/support/q8_k.rs` for Q8_K comparison-output
  parsing helpers.
- `crates/ferrite-cli/tests/support/mod.rs` as the integration-test support
  module boundary.

This reduces `next_token_cli.rs` from 832 lines to 749 lines and keeps the new
support modules small and focused.

## Verification

Commands:

```sh
cargo test -p ferrite-cli --test next_token_cli -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-cli --all-targets -- -D warnings
```

Observed results:

- `cargo test -p ferrite-cli --test next_token_cli -- --nocapture`: 25 passed,
  0 failed, 0 ignored, finished in 0.21s.
- `cargo fmt --all -- --check`: passed after applying rustfmt.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-cli --all-targets -- -D warnings`:
  passed in 6.77s.

## Result

The CLI next-token integration coverage remains intact while shared test
helpers now live in small support modules.

## Limits

This is a structure-only refactor. It does not add new CLI behavior, new model
coverage, or new OpenAI-compatible endpoint assertions.
