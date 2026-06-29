# Project Ferrite

Ferrite is a CPU native LLM inference engine made in Rust.

## OpenAI-Compatible Server

Ferrite includes a local OpenAI-compatible HTTP server:

```sh
cargo run --release -p ferrite-server -- \
  --model target/models/model.gguf \
  --model-id ferrite-local \
  --bind 127.0.0.1:8080 \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Initial endpoints:

- `GET /health`
- `GET /v1/models`
- `GET /v1/models/{model}`
- `POST /v1/completions`
- `POST /v1/chat/completions`

Example with the locally proven Qwen2.5-1.5B Q8_0 artifact:

```sh
cargo run --release -p ferrite-server -- \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0 \
  --bind 127.0.0.1:8080 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness and model catalog checks:

```sh
curl http://127.0.0.1:8080/health

curl http://127.0.0.1:8080/v1/models \
  -H 'authorization: Bearer local-secret'
```

Example completion request:

```sh
curl http://127.0.0.1:8080/v1/completions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer local-secret' \
  -d '{"model":"ferrite-local","prompt":"hello world","max_tokens":16}'
```

Example chat request:

```sh
curl http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer local-secret' \
  -d '{"model":"ferrite-local","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":16}'
```

Point OpenAI-compatible clients at `http://127.0.0.1:8080/v1` as the base URL.
The server supports non-streaming text generation and OpenAI-style SSE streams.
Ferrite has a live regression test using the `async-openai` client configured
with a Ferrite base URL.
`--api-key` is optional; when set, `/v1/*` endpoints require
`Authorization: Bearer <api-key>`, while `/health` remains open for local
readiness checks.
`--default-max-tokens` controls requests that omit `max_tokens` or
`max_completion_tokens`; `--hard-max-tokens` caps every generation request.
`--inference-wait-ms` controls how long an overlapping generation request waits
for the single inference permit before Ferrite returns an OpenAI-shaped
`429 rate_limit_error`. The default is `0`, which preserves immediate
backpressure. Ferrite does not yet execute multiple model generations in
parallel.

Streaming responses send token chunks as generation progresses:

```sh
curl -N http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer local-secret' \
  -d '{"model":"ferrite-local","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":16,"stream":true,"stream_options":{"include_usage":true}}'
```

When `stream_options.include_usage` is true, Ferrite emits a final usage chunk
before `data: [DONE]`.
