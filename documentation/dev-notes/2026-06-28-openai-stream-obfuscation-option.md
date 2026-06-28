# OpenAI Stream Obfuscation Option

Date: 2026-06-28

## Summary

Ferrite now treats `stream_options.include_obfuscation` precisely instead of
rejecting the field unconditionally.

`include_obfuscation: false` is accepted as a harmless no-op because Ferrite
does not emit obfuscation fields. `include_obfuscation: true` is still rejected
with an OpenAI-shaped invalid-request error because Ferrite does not implement
stream obfuscation.

## Implementation Notes

- Added `include_obfuscation` parsing to
  `crates/ferrite-server/src/openai/schema/stream_options.rs`.
- Kept enabled obfuscation on the unsupported-field path.
- Added focused chat acceptance and completion rejection tests in
  `crates/ferrite-server/src/openai/stream_options_tests.rs`.

## Verification

Red test first:

```sh
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
```

Initial result before implementation:

- `include_obfuscation: true` was rejected, as desired.
- `include_obfuscation: false` also returned `400`, proving the compatibility
  gap.

Final verification:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server openai::stream_options_tests -- --nocapture`:
  4 focused tests passed.
- `cargo test -p ferrite-server -- --nocapture`: 44 unit tests passed,
  3 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
