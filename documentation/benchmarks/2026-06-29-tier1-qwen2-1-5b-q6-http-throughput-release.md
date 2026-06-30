# Tier 1 Qwen2.5-1.5B Q6_K Release HTTP Throughput

Date: 2026-06-29

## Purpose

Measure a bounded release-build OpenAI-compatible HTTP request-rate sample for
Qwen2.5-1.5B-Instruct Q6_K using Ferrite's standalone server and throughput
client binaries.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt, one generated token per request, and the legacy
completion endpoint. It does not prove multi-model Tier 1 throughput, long
generation throughput, chat throughput, streaming throughput, x86_64 behavior,
multi-process load, long-running steady state, or production SLOs.

## Environment

- Commit: `d6e1393`
- Hardware: Apple M1 Pro
- CPU count: 8
- Memory: 17179869184 bytes
- OS: macOS 14.5 23F79
- Kernel: `Darwin Victors-MBP.localdomain 23.5.0 Darwin Kernel Version 23.5.0: Wed May  1 20:12:58 PDT 2024; root:xnu-10063.121.3~5/RELEASE_ARM64_T6000 arm64`
- Execution target: local macOS
- Build mode: Cargo release profile

## Model

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Server model ID: `qwen2.5-1.5b-q6_k`
- Endpoint: `POST /v1/completions`
- Prompt: `hello world`
- Generated tokens per request: 1

## Build

```sh
cargo build --release -p ferrite-server
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.05s
```

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k \
  --bind 127.0.0.1:18081 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18081/health
```

Result:

```json
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18081 \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=1
elapsed_ms=846
requests_per_second=1.180954
```

## Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18081 \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=10
elapsed_ms=8236
requests_per_second=1.214064
```

## Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18081 \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=9
elapsed_ms=7401
requests_per_second=1.215989
```

After the benchmark, `lsof -nP -iTCP:18081 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible legacy completion path can serve bounded
one-token Qwen2.5-1.5B Q6_K HTTP requests at about 1.21 requests per second on
this local Apple M1 Pro host. The queued three-client shape measured essentially
the same aggregate request rate because model execution remains serialized by
the single inference permit.

This adds release binary request-rate evidence for the second local
Qwen2.5-1.5B quantization. Broader HTTP throughput remains open: chat,
streaming, longer generations, multiple prompts, SmolLM2-1.7B, Qwen2.5-0.5B,
x86_64, longer steady-state runs, and memory sampling all need separate
evidence before Tier 1 HTTP throughput can be treated as complete.

## Current-Tree Rerun: 2026-06-30

This rerun repeats the same bounded release-build request shape on the current
tree after additional OpenAI-compatible endpoint hardening. It remains a narrow
one-host, one-model, one-prompt, one-token measurement and does not close the
broader Tier 1 HTTP throughput gate.

### Environment

- Commit: `75e6a99`
- Hardware: Apple M1 Pro
- CPU count: 8
- Memory: 17179869184 bytes
- OS: macOS 14.5 23F79
- Kernel: `Darwin Victors-MacBook-Pro.local 23.5.0 Darwin Kernel Version 23.5.0: Wed May  1 20:12:58 PDT 2024; root:xnu-10063.121.3~5/RELEASE_ARM64_T6000 arm64`
- Execution target: local macOS
- Build mode: Cargo release profile

### Build

```sh
cargo build --release -p ferrite-server
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.16s
```

### Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k \
  --bind 127.0.0.1:18081 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18081/health
```

Result:

```json
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18081 \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=1
elapsed_ms=968
requests_per_second=1.032771
```

### Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18081 \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=10
elapsed_ms=8394
requests_per_second=1.191309
```

### Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18081 \
  --model qwen2.5-1.5b-q6_k \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=9
elapsed_ms=7531
requests_per_second=1.195032
```

After the benchmark, `lsof -nP -iTCP:18081 -sTCP:LISTEN` returned no listener.

### Interpretation

The current tree preserves the earlier release-build result shape: bounded
one-token Qwen2.5-1.5B Q6_K completion requests measured about 1.19 requests
per second on this local Apple M1 Pro host. This rerun strengthens the release
evidence for the legacy completion endpoint only; chat, streaming, longer
generations, additional Tier 1 models, x86_64 behavior, steady-state load, and
memory sampling remain separate proof obligations.
