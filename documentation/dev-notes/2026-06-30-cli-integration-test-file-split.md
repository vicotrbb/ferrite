# CLI Integration Test File Split

Date: 2026-06-30

## Scope

This slice splits the large CLI integration test file by behavior without
changing command coverage or assertions.

The refactor keeps next-token basics in:

- `crates/ferrite-cli/tests/next_token_cli.rs`

It adds focused integration test targets for:

- `crates/ferrite-cli/tests/q8_k_cli.rs`
- `crates/ferrite-cli/tests/profile_cli.rs`
- `crates/ferrite-cli/tests/generation_cli.rs`

The existing shared test support remains under
`crates/ferrite-cli/tests/support/`. Because each integration test target is a
separate crate and uses a different subset of the shared helpers,
`tests/support/mod.rs` now allows `dead_code` locally for that support module.

This reduces `next_token_cli.rs` from 748 lines to 166 lines. The new focused
test files are 197, 183, and 219 lines.

## Verification

Commands:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-cli --test next_token_cli -- --nocapture
cargo test -p ferrite-cli --test q8_k_cli -- --nocapture
cargo test -p ferrite-cli --test profile_cli -- --nocapture
cargo test -p ferrite-cli --test generation_cli -- --nocapture
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-cli --all-targets -- -D warnings
```

Observed results:

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo test -p ferrite-cli --test next_token_cli -- --nocapture`: 6 passed,
  0 failed, 0 ignored, finished in 0.01s.
- `cargo test -p ferrite-cli --test q8_k_cli -- --nocapture`: 6 passed,
  0 failed, 0 ignored, finished in 31.23s.
- `cargo test -p ferrite-cli --test profile_cli -- --nocapture`: 5 passed,
  0 failed, 0 ignored, finished in 0.02s.
- `cargo test -p ferrite-cli --test generation_cli -- --nocapture`: 8 passed,
  0 failed, 0 ignored, finished in 0.24s.
- `CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-cli --all-targets -- -D warnings`:
  passed in 25.69s.

## Result

The CLI integration coverage is now organized by user-facing behavior while
preserving the same 25 CLI tests.

## Limits

This is a structure-only refactor. It does not add new CLI behavior, new model
coverage, or new OpenAI-compatible HTTP assertions.
