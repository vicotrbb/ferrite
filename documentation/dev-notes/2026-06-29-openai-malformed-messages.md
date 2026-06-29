# OpenAI Malformed Messages Field

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat completions endpoint now rejects malformed
top-level `messages` values through the normal OpenAI-shaped validation path
instead of a generic JSON body deserialization failure.

This applies to:

- `POST /v1/chat/completions` with `messages: null`
- `POST /v1/chat/completions` with non-array `messages`

OpenAI documents chat input as a `messages` array. Ferrite still requires at
least one usable local text transcript message, but malformed top-level values
now produce a structured `invalid_request_error` that mentions `messages`.

Source reference:

- https://developers.openai.com/api/reference/resources/chat

## Red

```sh
cargo test -p ferrite-server messages -- --nocapture
```

Before implementation, the two new route tests failed because request-body
deserialization stopped before Ferrite could return a structured
OpenAI-compatible error object.

Observed failures:

```text
Failed to deserialize the JSON body into the target type
```

The failing tests were:

- `chat_endpoint_rejects_null_messages`
- `chat_endpoint_rejects_non_array_messages`

## Green

Added a focused `openai::schema::chat_messages` parser used by the chat
completion request schema. Valid message arrays are preserved. Null, non-array,
or otherwise unparseable message payloads become empty internal input so the
existing prompt validation returns `400 invalid_request_error` before inference:

```text
messages must contain at least one item
```

Focused check:

```sh
cargo test -p ferrite-server messages -- --nocapture
```

Observed result:

- 7 matching unit, prompt, and route tests passed.

## Limits

This slice does not infer chat messages, synthesize prompts, or add multimodal
message execution. Malformed chat input remains invalid and is rejected before
model execution.
