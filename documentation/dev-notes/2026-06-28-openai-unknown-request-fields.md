# OpenAI Unknown Request Fields

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible chat and legacy completion endpoints now reject
unknown request fields instead of letting Serde silently discard them.

This protects the local server contract from pretending to accept newer OpenAI
parameters, provider-specific options, or misspelled fields that Ferrite does
not implement.

## Implementation Notes

- Added flattened `extra_fields` capture to `ChatCompletionRequest`.
- Added flattened `extra_fields` capture to `CompletionRequest`.
- Extra field names are appended to the existing unsupported-field error path.
- Added focused tests for unknown chat and completion request fields.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Initial result before implementation:

- `chat_endpoint_rejects_unknown_fields` returned `503` instead of `400`.
- `completion_endpoint_rejects_unknown_fields` returned `503` instead of `400`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 31 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
