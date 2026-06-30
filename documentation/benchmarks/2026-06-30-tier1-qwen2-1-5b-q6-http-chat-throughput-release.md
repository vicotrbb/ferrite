# Tier 1 Qwen2.5-1.5B Q6_K Release HTTP Chat Throughput

Date: 2026-06-30

## Purpose

Measure a bounded release-build OpenAI-compatible HTTP request-rate sample for
Qwen2.5-1.5B-Instruct Q6_K using Ferrite's standalone server and throughput
client binaries against `POST /v1/chat/completions`.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt wrapped as a single user chat message, one generated
token per request, and the non-streaming chat endpoint. It does not prove
multi-model Tier 1 throughput, long generation throughput, legacy completion
throughput, streaming throughput, x86_64 behavior, multi-process load,
long-running steady state, or production SLOs.

## Environment

- Commit: `9a80566`
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
- Endpoint: `POST /v1/chat/completions`
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
  --bind 127.0.0.1:18089 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18089/health
```

Result:

```json
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18089 \
  --endpoint chat-completions \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=1
elapsed_ms=2544
requests_per_second=0.392977
```

## Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18089 \
  --endpoint chat-completions \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=10
elapsed_ms=23894
requests_per_second=0.418504
```

## Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18089 \
  --endpoint chat-completions \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=9
elapsed_ms=22073
requests_per_second=0.407721
```

After the benchmark, `lsof -nP -iTCP:18089 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible non-streaming chat path can serve bounded
one-token Qwen2.5-1.5B Q6_K chat requests at about 0.42 requests per second on
this local Apple M1 Pro host. The queued three-client shape measured about
0.41 requests per second because model execution remains serialized by the
single inference permit.

This adds release binary request-rate evidence for the second local
Qwen2.5-1.5B chat quantization. Broader HTTP throughput remains open: SmolLM2
chat, longer generations, multiple prompts, x86_64, longer steady-state runs,
and memory sampling all need separate evidence before Tier 1 HTTP throughput
can be treated as complete.
