# OpenAI Model Param Error Shape

## Context

Ferrite's OpenAI-compatible completion endpoints rejected requests without a
usable `model`, but the error body did not identify the failed parameter. Common
OpenAI clients and curl debugging workflows benefit from the structured
`error.param` field when request validation fails.

## Change

Missing or non-string model ids now return an `invalid_request_error` with
`error.param` set to `model` on both `/v1/completions` and
`/v1/chat/completions`.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-server openai::availability_tests::endpoints_report_model_param_when_model_is_missing -- --nocapture
```
