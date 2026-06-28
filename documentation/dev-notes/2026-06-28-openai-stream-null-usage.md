# OpenAI Stream Null Usage Chunks

Date: 2026-06-28

## Summary

Ferrite now mirrors the OpenAI streaming usage contract more closely when
`stream_options.include_usage` is requested.

Normal chat and completion stream chunks include `usage: null`, the final
empty-choices chunk includes the actual usage object, and the stream still ends
with `data: [DONE]`.

## Implementation Notes

- Added `crates/ferrite-server/src/openai/schema/stream_usage.rs` so stream
  chunks can distinguish omitted usage from explicit JSON `null`.
- Added `with_usage_field(...)` to chat and completion stream contexts.
- Kept default streaming behavior unchanged when usage is not requested.
- Strengthened focused stream-options tests to assert token chunks include
  `usage:null`.

## Verification

Red test first:

```sh
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
```

Initial result before implementation:

- both stream-options tests failed because `usage:null` was absent from normal
  token chunks.

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
  2 focused tests passed.
- `cargo test -p ferrite-server -- --nocapture`: 42 unit tests passed,
  3 `openai_client` integration tests passed, and 2 `openai_http` integration
  tests passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
