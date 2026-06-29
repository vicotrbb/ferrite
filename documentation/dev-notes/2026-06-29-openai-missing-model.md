# OpenAI Missing Model Field

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible generation endpoints now reject requests that omit
the top-level `model` field with an OpenAI-shaped `invalid_request_error`
instead of a generic JSON body deserialization failure.

This applies to:

- `POST /v1/chat/completions`
- `POST /v1/completions`

OpenAI documents `model` as a body parameter for both generation endpoints, so
Ferrite keeps the field required while reporting the error through the same
structured response shape as other compatibility validation.

Source references:

- https://developers.openai.com/api/reference/resources/chat
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Red

```sh
cargo test -p ferrite-server missing_model -- --nocapture
```

Before implementation, both new tests failed because request-body
deserialization stopped on the missing `model` field before route validation
could return a structured OpenAI-shaped error.

Observed failure:

```text
Failed to deserialize the JSON body into the target type
```

## Green

The request schemas now default a missing `model` to an empty internal string
so route validation can decide the public error shape. The shared generation
model check returns `400 invalid_request_error` for an empty model id and keeps
the existing `404 model_not_found` behavior for non-empty unknown model ids.

Focused check:

```sh
cargo test -p ferrite-server missing_model -- --nocapture
```

Observed result:

- `chat_endpoint_rejects_missing_model` passed.
- `completion_endpoint_rejects_missing_model` passed.

## Limits

This slice does not add model aliases, model auto-selection, or multi-model
routing. Requests still must name the loaded local model to reach inference.
