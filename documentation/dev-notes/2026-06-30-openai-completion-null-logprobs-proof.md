# OpenAI completion null logprobs proof

## Context

Legacy OpenAI completion requests may include optional fields as JSON `null`.
Ferrite rejects unsupported `logprobs` requests because log probability output
is not implemented, but a `null` value is only an explicit absence and should
remain compatible with OpenAI-style JSON clients.

## Slice

Add focused coverage proving `POST /v1/completions` accepts
`"logprobs": null` as neutral request input.

The test passed against the existing schema because Serde deserializes JSON
`null` into `None` for `Option<Value>`. No production-code change was required.

## Validation

Executed:

- `cargo test -p ferrite-server --lib openai::completion_option_tests::completions_endpoint_accepts_null_logprobs -- --nocapture`
