# OpenAI Malformed Prompt Field

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible legacy completions endpoint now rejects malformed
top-level `prompt` values through the normal OpenAI-shaped validation path
instead of a generic JSON body deserialization failure.

This applies to:

- `POST /v1/completions` with `prompt: null`
- `POST /v1/completions` with object `prompt` values

OpenAI documents legacy completion `prompt` as text, arrays of text, token ids,
or token-id arrays. Ferrite's local execution path supports text and arrays of
text; token forms and malformed forms are explicit `prompt` validation errors.

Source reference:

- https://developers.openai.com/api/reference/resources/completions/methods/create

## Red

```sh
cargo test -p ferrite-server prompt -- --nocapture
```

Before implementation, the two new route tests failed because request-body
deserialization stopped before Ferrite could return a structured
OpenAI-compatible error object.

Observed failures:

```text
Failed to deserialize the JSON body into the target type
```

The failing tests were:

- `completion_endpoint_rejects_null_prompt`
- `completion_endpoint_rejects_object_prompt`

## Green

Updated `openai::schema::completion_prompt` to parse through `serde_json::Value`
and classify supported text forms, token prompt forms, and malformed forms
explicitly. Supported text and text-array prompts are preserved. Token arrays,
null, object, and other unsupported wire shapes become empty internal prompt
input with `has_unsupported_form = true`, so route validation returns
`400 invalid_request_error` for `prompt` before inference.

Focused check:

```sh
cargo test -p ferrite-server prompt -- --nocapture
```

Observed result:

- 18 matching schema, route, and prompt tests passed.

## Limits

This slice does not implement token-id prompt execution, prompt coercion, or
object prompt forms. Malformed prompt input remains invalid and is rejected
before model execution.
