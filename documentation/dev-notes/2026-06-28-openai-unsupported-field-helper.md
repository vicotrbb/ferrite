# OpenAI Unsupported Field Helper

Date: 2026-06-28

## Summary

Unsupported OpenAI request-field collection is now shared by chat and legacy
completion schemas through `crates/ferrite-server/src/openai/schema/unsupported.rs`.

This keeps the growing compatibility guardrails out of individual request
schemas and avoids duplicating the same list-building pattern across endpoints.

## Implementation Notes

- Added `UnsupportedFields`.
- Added a focused helper test covering named fields plus flattened extra keys.
- Updated `ChatCompletionRequest::unsupported_fields()` and
  `CompletionRequest::unsupported_fields()` to use the helper.

## Verification

Red test first:

```sh
cargo test -p ferrite-server openai::schema::unsupported::tests::collects_named_and_extra_unsupported_fields -- --nocapture
```

Initial result before implementation:

- compile failed because `UnsupportedFields` did not exist.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 32 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
