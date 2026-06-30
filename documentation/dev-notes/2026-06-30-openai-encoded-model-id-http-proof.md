# OpenAI Encoded Model ID HTTP Proof

## Scope

This slice proves Ferrite's raw live HTTP model-retrieve path accepts
percent-encoded slashes in model IDs.

Model IDs commonly use provider-style names such as
`HuggingFaceTB/SmolLM2-135M-Instruct`. Ferrite already had a focused route test
for encoded slash retrieval; this adds live HTTP coverage through the same
socket-level smoke suite that covers `/v1/models`, completions, chat, streaming,
and bearer-token auth.

## Red

The new integration test first failed because the live-server test support could
not start the fixture model with an arbitrary model ID:

```text
no associated function or constant named `start_with_model_id` found for struct `LiveServer`
```

## Green

- Added `LiveServer::start_with_model_id` to the HTTP integration-test support.
- Added `live_http_server_retrieves_encoded_slash_model_id` to
  `crates/ferrite-server/tests/openai_http.rs`.
- The test sends
  `GET /v1/models/HuggingFaceTB%2FSmolLM2-135M-Instruct` over a live TCP
  connection and verifies the OpenAI-shaped model response.

Focused check:

```sh
cargo test -p ferrite-server --test openai_http live_http_server_retrieves_encoded_slash_model_id -- --nocapture
```

## Interpretation

This is fixture-backed protocol evidence for the OpenAI-compatible local server
catalog path. It does not add new generation behavior or broader OpenAI API
parity.
