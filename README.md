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

curl http://127.0.0.1:8080/v1/models/ferrite-local \
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
Ferrite has live regression tests using the `async-openai` client configured
with a Ferrite base URL for model catalog, legacy completions, chat
completions, SSE streams, and bearer-token auth.
Ferrite returns CORS headers for `/v1/*` responses and unauthenticated CORS
preflight responses for supported OpenAI-compatible endpoints so local browser
clients can call the API.
`--api-key` is optional; when set, `/v1/*` endpoints require
`Authorization: Bearer <api-key>`, while `/health` remains open for local
readiness checks. `/health` returns `ready: false` until a model is loaded.
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

Release-oriented HTTP throughput checks should run the server and benchmark
client as separate release binaries:

```sh
cargo build --release -p ferrite-server

target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0 \
  --bind 127.0.0.1:8080 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000

target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:8080 \
  --endpoint completions \
  --model qwen2.5-1.5b-q8_0 \
  --prompt 'hello world' \
  --requests 3 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Use `--endpoint chat-completions` to measure
`POST /v1/chat/completions` with the prompt wrapped as a single user message.
Use `--stream` to measure OpenAI-style SSE streams for either endpoint. The
client prints endpoint-specific request counters:
`openai_http_completion_requests`, `openai_http_chat_completion_requests`,
`openai_http_streaming_completion_requests`, or
`openai_http_streaming_chat_completion_requests`, plus `elapsed_ms` and
`requests_per_second`. Record throughput claims under
`documentation/benchmarks/` with the exact server/client commands, model, host,
build mode, endpoint, stream mode, request count, concurrency, prompt, and
generated-token count.

## CLI Memory Sampling

For memory probes, the `ferrite` CLI can pause after loading the model and
dropping the raw GGUF byte buffer:

```sh
target/release/ferrite \
  --model target/models/model.gguf \
  --prompt 'hello world' \
  --sleep-after-load-ms 5000 \
  --generate-tokens 1
```

The CLI prints `sleep_after_load_ms=<ms>` and flushes stdout before sleeping,
so an external sampler can collect current RSS with `ps -o rss= -p "$pid"`.
Use `/usr/bin/time -l` separately for peak RSS; wrapping the command with
`time` changes which process `$!` points at in shell scripts.
