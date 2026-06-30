# Tier 1 Qwen2.5-0.5B Q4_K_M Release HTTP Chat Throughput

Date: 2026-06-30

## Purpose

Measure a bounded release-build OpenAI-compatible HTTP request-rate sample for
Qwen2.5-0.5B-Instruct Q4_K_M using Ferrite's standalone server and throughput
client binaries against `POST /v1/chat/completions`.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt wrapped as a single user chat message, one generated
token per request, and the non-streaming chat endpoint. It does not prove
multi-model Tier 1 throughput, long generation throughput, legacy completion
throughput, streaming throughput, x86_64 behavior, multi-process load,
long-running steady state, or production SLOs.

## Environment

- Commit: `2a1dbcd`
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
- Endpoint: `POST /v1/chat/completions`
- Prompt: `hello world`
- Generated tokens per request: 1

## Build

```sh
cargo build --release -p ferrite-server
```

Result:

```text
Finished `release` profile [optimized] target(s) in 3.89s
```

## Server

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --bind 127.0.0.1:18084 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18084/health
```

Result:

```json
{"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18084 \
  --endpoint chat-completions \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=1
elapsed_ms=411
requests_per_second=2.432655
```

## Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18084 \
  --endpoint chat-completions \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=10
elapsed_ms=3970
requests_per_second=2.518772
```

## Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18084 \
  --endpoint chat-completions \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=9
elapsed_ms=3603
requests_per_second=2.497655
```

After the benchmark, `lsof -nP -iTCP:18084 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible non-streaming chat path can serve bounded
one-token Qwen2.5-0.5B Q4_K_M chat requests at about 2.5 requests per second on
this local Apple M1 Pro host. The queued three-client shape measured
essentially the same aggregate request rate because model execution remains
serialized by the single inference permit.

This adds release binary request-rate evidence for one Tier 1 chat completion
shape. Broader HTTP throughput remains open: additional Tier 1 chat models,
legacy completion comparisons, streaming, longer generations, multiple prompts,
x86_64, longer steady-state runs, and memory sampling all need separate
evidence before Tier 1 HTTP throughput can be treated as complete.
