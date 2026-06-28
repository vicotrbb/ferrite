# OpenAI Unsupported Chat Sampling Fields

Date: 2026-06-28

## Summary

Ferrite's OpenAI-compatible chat endpoint now rejects additional unsupported
fields that would change sampling, choice count, stopping, logging, or storage
semantics.

Rejected fields now include:

- `temperature`
- `top_p`
- `n`
- `stop`
- `presence_penalty`
- `frequency_penalty`
- `logit_bias`
- `logprobs`
- `top_logprobs`
- `user`
- `seed`
- `stream_options`
- `store`
- `metadata`

This extends the earlier chat rejection for tool and structured-output fields,
and keeps `POST /v1/chat/completions` from silently pretending to honor OpenAI
features Ferrite has not implemented.

## Implementation Notes

- Extended `ChatCompletionRequest::unsupported_fields()`.
- Added focused tests for `temperature` and `n` in
  `crates/ferrite-server/src/openai/unsupported_tests.rs`.
- Left supported local-serving fields unchanged: `model`, `messages`, `stream`,
  `max_tokens`, and `max_completion_tokens`.

## Verification

Red tests first:

```sh
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Initial result before implementation:

- `chat_endpoint_rejects_sampling_parameters` returned `503` instead of `400`.
- `chat_endpoint_rejects_multiple_choice_request` returned `503` instead of
  `400`.

Final verification:

```sh
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 29 unit tests passed,
  `async_openai_client_uses_ferrite_base_url` passed, and
  `live_http_server_accepts_openai_style_chat_request` passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `cargo check --workspace`: passed.
- `git diff --check`: passed.
