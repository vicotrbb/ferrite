# OpenAI Direct Model Retrieve Proof

## Slice

Ferrite already exposed `GET /v1/models/{model}` and had focused route tests
plus an `async-openai` client retrieval proof. This slice adds direct live HTTP
coverage for the same endpoint so the local OpenAI-compatible server smoke
suite covers every initial catalog and generation route through raw HTTP.

## Change

- Added `live_http_server_accepts_openai_style_model_retrieve` to
  `crates/ferrite-server/tests/openai_http.rs`.
- The test starts a live Ferrite fixture server, sends
  `GET /v1/models/{model}`, and verifies the OpenAI-shaped model object
  response.
- Added the matching `curl` example to `README.md` so local OpenAI-compatible
  server setup documents both model-list and model-retrieve catalog checks.

## Validation

```sh
cargo test -p ferrite-server --test openai_http live_http_server_accepts_openai_style_model_retrieve -- --nocapture
cargo test -p ferrite-server --test openai_http -- --nocapture
```

This is fixture-backed compatibility evidence. It does not add new API scope or
claim broader OpenAI API parity beyond Ferrite's local text-generation subset.
