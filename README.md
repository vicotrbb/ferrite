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
- `POST /v1/completions`
- `POST /v1/chat/completions`

Example completion request:

```sh
curl http://127.0.0.1:8080/v1/completions \
  -H 'content-type: application/json' \
  -d '{"model":"ferrite-local","prompt":"hello world","max_tokens":16}'
```

Point OpenAI-compatible clients at `http://127.0.0.1:8080/v1` as the base URL.
The first server slice supports non-streaming text generation. SSE streaming is
tracked as the next compatibility slice.
