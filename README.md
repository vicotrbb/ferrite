# Project Ferrite

Ferrite is a CPU native LLM inference engine made in Rust.

## OpenAI-Compatible Server

Ferrite includes a local OpenAI-compatible HTTP server:

```sh
cargo run -p ferrite-server -- \
  --model target/models/model.gguf \
  --model-id ferrite-local \
  --bind 127.0.0.1:8080
```

Initial endpoints:

- `GET /health`
- `GET /v1/models`
- `GET /v1/models/{model}`
- `POST /v1/completions`
- `POST /v1/chat/completions`

Example completion request:

```sh
curl http://127.0.0.1:8080/v1/completions \
  -H 'content-type: application/json' \
  -d '{"model":"ferrite-local","prompt":"hello world","max_tokens":16}'
```

Point OpenAI-compatible clients at `http://127.0.0.1:8080/v1` as the base URL.
The server supports non-streaming text generation and OpenAI-style SSE streams.
Streaming responses send token chunks as generation progresses:

```sh
curl -N http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"ferrite-local","messages":[{"role":"user","content":"hello world"}],"max_tokens":16,"stream":true}'
```
