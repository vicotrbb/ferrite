# OpenAI Malformed Model Field

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible generation endpoints now reject malformed top-level
`model` values through the normal OpenAI-shaped validation path instead of a
generic JSON body deserialization failure.

This applies to:

- `POST /v1/chat/completions`
- `POST /v1/completions`

OpenAI documents `model` as a string-like model id. Ferrite still requires a
string id naming the loaded local model, but malformed values now produce a
structured `invalid_request_error` that mentions `model`.

Source references:

- https://developers.openai.com/api/reference/resources/chat
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Red

```sh
cargo test -p ferrite-server model -- --nocapture
```

Before implementation, the new malformed-model route tests failed because
request-body deserialization stopped before Ferrite could return a structured
OpenAI-compatible error object.

Observed failures:

```text
Failed to deserialize the JSON body into the target type
```

The failing tests were:

- `chat_endpoint_rejects_non_string_model`
- `completion_endpoint_rejects_null_model`

## Green

Added a focused `openai::schema::model_id` parser used by both generation
request schemas. Valid string model ids are preserved. Null and non-string
model values become an empty internal value so the existing shared
`ensure_model` validation returns `400 invalid_request_error` before inference.

Focused check:

```sh
cargo test -p ferrite-server model -- --nocapture
```

Observed result:

- 19 matching unit, route, and live fixture-server tests passed.

## Limits

This slice does not add model aliases, automatic model selection, or multi-model
routing. Non-empty unknown string model ids still return `404 model_not_found`.
