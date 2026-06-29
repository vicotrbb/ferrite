# OpenAI Missing Generation Inputs

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible generation endpoints now reject requests that omit
their top-level generation input fields with OpenAI-shaped
`invalid_request_error` responses instead of generic JSON body deserialization
failures.

This applies to:

- `POST /v1/chat/completions` without `messages`
- `POST /v1/completions` without `prompt`

The fields remain required for useful local inference. This slice only moves
the public error shape into Ferrite's route validation path so OpenAI-style
clients receive structured errors that name the relevant request field.

Source references:

- https://developers.openai.com/api/reference/resources/chat
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Red

```sh
cargo test -p ferrite-server missing_ -- --nocapture
```

Before implementation, the two new route tests failed because request-body
deserialization stopped on the missing top-level field before Ferrite could
return its structured OpenAI-compatible error object.

Observed failures:

```text
Failed to deserialize the JSON body into the target type
```

The failing tests were:

- `chat_endpoint_rejects_missing_messages`
- `completion_endpoint_rejects_missing_prompt`

## Green

The chat request schema now defaults an omitted `messages` field to an empty
vector, and the completion request schema defaults an omitted `prompt` field to
an empty prompt marker. Existing route validation then rejects those empty
inputs before inference:

- `messages must contain at least one item`
- `prompt must contain at least one item`

Focused check:

```sh
cargo test -p ferrite-server missing_ -- --nocapture
```

Observed result:

- 23 matching tests passed.

## Limits

This slice does not infer a prompt, synthesize a chat transcript, or change the
supported local text-generation input forms. Missing generation input remains
invalid and is rejected before model execution.
