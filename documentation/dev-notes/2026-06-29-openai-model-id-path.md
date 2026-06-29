# OpenAI model catalog path compatibility

## Scope

This slice locks the OpenAI-compatible model retrieve endpoint behavior for
URL-encoded slash-bearing model IDs such as
`HuggingFaceTB/SmolLM2-135M-Instruct`.

Open-weight model names often use `provider/model` IDs. Request bodies already
carry model IDs as JSON strings, but `GET /v1/models/{model}` also needs to
accept the same IDs when the slash is URL-encoded by an OpenAI-compatible
client.

## Evidence

- Added `model_retrieve_endpoint_supports_encoded_slashes` in
  `crates/ferrite-server/src/openai/catalog_tests.rs`.
- Ran `cargo test -p ferrite-server openai::catalog_tests -- --nocapture`.
- Result: 5 passed; the existing route and `Path<String>` extractor already
  decode `HuggingFaceTB%2FSmolLM2-135M-Instruct` to the configured model ID.

## Boundary

This verifies encoded slash compatibility for `GET /v1/models/{model}`. It
does not claim support for unencoded raw slash path segments in model IDs; HTTP
clients should percent-encode the model ID path parameter.
