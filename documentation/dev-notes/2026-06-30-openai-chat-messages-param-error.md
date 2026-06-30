# OpenAI Chat Messages Param Error

## Context

Ferrite's OpenAI-compatible chat completions route rejected empty `messages`
arrays, but the route-level validation error did not populate `error.param`.
The message named the field, but the structured parameter value is useful for
OpenAI-compatible clients and direct curl debugging.

## Change

Empty chat message arrays now return an `invalid_request_error` with
`error.param` set to `messages`.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-server chat_endpoint_reports_messages_param_for_empty_messages -- --nocapture
```
