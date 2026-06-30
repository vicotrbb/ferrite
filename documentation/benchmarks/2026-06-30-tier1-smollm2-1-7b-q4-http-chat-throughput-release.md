# Tier 1 SmolLM2-1.7B Q4_K_M Release HTTP Chat Throughput

Date: 2026-06-30

## Purpose

Measure a bounded release-build OpenAI-compatible HTTP request-rate sample for
SmolLM2-1.7B-Instruct Q4_K_M using Ferrite's standalone server and throughput
client binaries against `POST /v1/chat/completions`.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt wrapped as a single user chat message, one generated
token per request, and the non-streaming chat endpoint. It does not prove
multi-model Tier 1 throughput, long generation throughput, legacy completion
throughput, streaming throughput, x86_64 behavior, multi-process load,
long-running steady state, or production SLOs.

## Environment

- Commit: `ce6a4d2`
- Hardware: Apple M1 Pro
- CPU count: 8
- Memory: 17179869184 bytes
- OS: macOS 14.5 23F79
- Kernel: `Darwin Victors-MacBook-Pro.local 23.5.0 Darwin Kernel Version 23.5.0: Wed May  1 20:12:58 PDT 2024; root:xnu-10063.121.3~5/RELEASE_ARM64_T6000 arm64`
- Execution target: local macOS
- Build mode: Cargo release profile

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m`
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
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m \
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
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18089 \
  --endpoint chat-completions \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=1
elapsed_ms=1862
requests_per_second=0.536858
```

## Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18089 \
  --endpoint chat-completions \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=10
elapsed_ms=18139
requests_per_second=0.551279
```

## Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18089 \
  --endpoint chat-completions \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_chat_completion_requests=9
elapsed_ms=15708
requests_per_second=0.572925
```

After the benchmark, `lsof -nP -iTCP:18089 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible non-streaming chat path can serve bounded
one-token SmolLM2-1.7B Q4_K_M chat requests at about 0.55 requests per second
on this local Apple M1 Pro host. The queued three-client shape measured about
0.57 requests per second because model execution remains serialized by the
single inference permit.

This adds release binary request-rate evidence for the current local Tier 1
non-streaming chat model-family set. Broader HTTP throughput remains open:
longer generations, multiple prompts, x86_64, longer steady-state runs, and
memory sampling all need separate evidence before Tier 1 HTTP throughput can
be treated as complete.
