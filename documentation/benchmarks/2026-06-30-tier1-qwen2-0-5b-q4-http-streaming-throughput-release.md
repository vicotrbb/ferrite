# Tier 1 Qwen2.5-0.5B Q4_K_M Release HTTP Streaming Throughput

Date: 2026-06-30

## Purpose

Measure bounded release-build OpenAI-compatible HTTP request-rate samples for
Qwen2.5-0.5B-Instruct Q4_K_M using Ferrite's standalone server and throughput
client binaries against streaming SSE response paths.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt, one generated token per request, and the streaming
legacy completion plus streaming chat endpoints. It does not prove multi-model
Tier 1 throughput, long generation throughput, non-streaming throughput,
x86_64 behavior, multi-process load, long-running steady state, or production
SLOs.

## Environment

- Commit: `c677c6f`
- Hardware: Apple M1 Pro
- CPU count: 8
- Memory: 17179869184 bytes
- OS: macOS 14.5 23F79
- Kernel: `Darwin Victors-MacBook-Pro.local 23.5.0 Darwin Kernel Version 23.5.0: Wed May  1 20:12:58 PDT 2024; root:xnu-10063.121.3~5/RELEASE_ARM64_T6000 arm64`
- Execution target: local macOS
- Build mode: Cargo release profile

## Model

- Model: Qwen2.5-0.5B-Instruct Q4_K_M GGUF
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Server model ID: `qwen2.5-0.5b-q4_k_m`
- Endpoints: `POST /v1/completions`, `POST /v1/chat/completions`
- Stream mode: OpenAI-compatible SSE with terminal `data: [DONE]`
- Prompt: `hello world`
- Generated tokens per request: 1

## Build

```sh
cargo build --release -p ferrite-server
```

Result:

```text
Finished `release` profile [optimized] target(s) in 3.75s
```

## Server

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --bind 127.0.0.1:18086 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18086/health
```

Result:

```json
{"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

## Streaming Legacy Completion

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18086 \
  --endpoint completions \
  --stream \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_completion_requests=1
elapsed_ms=190
requests_per_second=5.249339
```

Sequential result:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18086 \
  --endpoint completions \
  --stream \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_completion_requests=10
elapsed_ms=1716
requests_per_second=5.826872
```

Queued result:

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18086 \
  --endpoint completions \
  --stream \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_completion_requests=9
elapsed_ms=1818
requests_per_second=4.947972
```

## Streaming Chat Completion

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18086 \
  --endpoint chat-completions \
  --stream \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_chat_completion_requests=1
elapsed_ms=395
requests_per_second=2.530314
```

Sequential result:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18086 \
  --endpoint chat-completions \
  --stream \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_chat_completion_requests=10
elapsed_ms=3917
requests_per_second=2.552487
```

Queued result:

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18086 \
  --endpoint chat-completions \
  --stream \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_chat_completion_requests=9
elapsed_ms=3499
requests_per_second=2.571900
```

After the benchmark, `lsof -nP -iTCP:18086 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible streaming legacy completion path can serve
bounded one-token Qwen2.5-0.5B Q4_K_M SSE requests at about 5.83 requests per
second sequentially and about 4.95 requests per second in the queued
three-client shape on this local Apple M1 Pro host. The streaming chat path can
serve the same bounded prompt at about 2.55 requests per second sequentially
and about 2.57 requests per second queued.

This adds release binary request-rate evidence for one Tier 1 streaming model
shape. Broader HTTP throughput remains open: additional Tier 1 streaming
models, longer generations, multiple prompts, x86_64, longer steady-state runs,
and memory sampling all need separate evidence before Tier 1 HTTP throughput
can be treated as complete.
