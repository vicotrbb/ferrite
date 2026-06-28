# OpenAI Unsupported Completion Fields

Date: 2026-06-28

## Summary

Ferrite's legacy OpenAI-compatible completion endpoint now rejects unsupported
request fields that would imply behavior the server does not implement.

Rejected fields currently include:

- `suffix`
- `temperature`
- `top_p`
- `n`
- `logprobs`
- `echo`
- `stop`
- `presence_penalty`
- `frequency_penalty`
- `best_of`
- `logit_bias`
- `user`

This keeps `POST /v1/completions` aligned with ADR 0008: unsupported OpenAI
fields are not silently ignored when honoring them would be misleading.

## Implementation Notes

- Added unsupported-field detection to `CompletionRequest`.
- Added a completion route guard before prompt validation and inference
  execution.
- Extended `crates/ferrite-server/src/openai/unsupported_tests.rs` with
  completion-specific cases for `n` and `logprobs`.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Initial result before implementation:

- `completion_endpoint_rejects_multiple_choice_request` returned `503` instead
  of `400`.
- `completion_endpoint_rejects_logprobs_request` returned `503` instead of
  `400`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 27 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
