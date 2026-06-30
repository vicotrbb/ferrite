# Tier 1 Qwen2.5-1.5B Q6_K Release HTTP Streaming Throughput

Date: 2026-06-30

## Purpose

Measure bounded release-build OpenAI-compatible HTTP request-rate samples for
Qwen2.5-1.5B-Instruct Q6_K using Ferrite's standalone server and throughput
client binaries against streaming SSE response paths.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt, one generated token per request, and the streaming
legacy completion plus streaming chat endpoints. It does not prove multi-model
Tier 1 throughput, long generation throughput, non-streaming throughput,
x86_64 behavior, multi-process load, long-running steady state, or production
SLOs.

## Environment

- Commit: `f65f294`
- Hardware: Apple M1 Pro
- CPU count: 8
- Memory: 17179869184 bytes
- OS: macOS 14.5 23F79
- Kernel: `Darwin Victors-MacBook-Pro.local 23.5.0 Darwin Kernel Version 23.5.0: Wed May  1 20:12:58 PDT 2024; root:xnu-10063.121.3~5/RELEASE_ARM64_T6000 arm64`
- Execution target: local macOS
- Build mode: Cargo release profile

## Model

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Server model ID: `qwen2.5-1.5b-q6_k`
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
Finished `release` profile [optimized] target(s) in 0.19s
```

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k \
  --bind 127.0.0.1:18088 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18088/health
```

Result:

```json
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k"}
```

## Streaming Legacy Completion

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18088 \
  --endpoint completions \
  --stream \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_completion_requests=1
elapsed_ms=1025
requests_per_second=0.975059
```

Sequential result:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18088 \
  --endpoint completions \
  --stream \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_completion_requests=10
elapsed_ms=8667
requests_per_second=1.153716
```

Queued result:

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18088 \
  --endpoint completions \
  --stream \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_completion_requests=9
elapsed_ms=7624
requests_per_second=1.180446
```

## Streaming Chat Completion

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18088 \
  --endpoint chat-completions \
  --stream \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_chat_completion_requests=1
elapsed_ms=2498
requests_per_second=0.400305
```

Sequential result:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18088 \
  --endpoint chat-completions \
  --stream \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_chat_completion_requests=10
elapsed_ms=24102
requests_per_second=0.414899
```

Queued result:

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18088 \
  --endpoint chat-completions \
  --stream \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_streaming_chat_completion_requests=9
elapsed_ms=22520
requests_per_second=0.399641
```

After the benchmark, `lsof -nP -iTCP:18088 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible streaming legacy completion path can serve
bounded one-token Qwen2.5-1.5B Q6_K SSE requests at about 1.15 requests per
second sequentially and about 1.18 requests per second in the queued
three-client shape on this local Apple M1 Pro host. The streaming chat path can
serve the same bounded prompt at about 0.41 requests per second sequentially
and about 0.40 requests per second queued.

This adds release binary request-rate evidence for the second local
Qwen2.5-1.5B streaming quantization. Broader HTTP throughput remains open:
SmolLM2 streaming, longer generations, multiple prompts, x86_64, longer
steady-state runs, and memory sampling all need separate evidence before Tier 1
HTTP throughput can be treated as complete.
