# OpenAI Malformed Stream Flag

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat and legacy completion endpoints now return an
explicit `stream` field error when clients provide a malformed stream flag.

This covers:

- `stream` on `POST /v1/chat/completions`
- `stream` on `POST /v1/completions`

## Red

Added route tests for string stream flags:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Initial failures:

```text
chat_endpoint_rejects_malformed_stream_flag
Failed to deserialize the JSON body into the target type

completion_endpoint_rejects_malformed_stream_flag
Failed to deserialize the JSON body into the target type
```

Both failures showed that the typed `bool` request field failed before the
normal OpenAI-shaped unsupported-field validation could name `stream`.

## Implementation

- Added `crates/ferrite-server/src/openai/schema/stream_flag.rs`.
- Parsed `stream` through a small schema type that preserves valid booleans,
  treats `null` as an omitted optional flag, and records malformed values.
- Wired chat and legacy completion request structs to report malformed stream
  flags as unsupported `stream` fields.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Result:

```text
53 passed; 0 failed; 0 ignored
```

## Limits

This slice does not change the SSE streaming implementation or generation
runtime behavior. It only ensures malformed `stream` values produce
OpenAI-shaped field errors instead of generic JSON body deserialization
failures.
