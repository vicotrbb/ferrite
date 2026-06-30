# Tier 1 SmolLM2-1.7B Q4_K_M Release HTTP Throughput

Date: 2026-06-29

## Purpose

Measure a bounded release-build OpenAI-compatible HTTP request-rate sample for
SmolLM2-1.7B-Instruct Q4_K_M using Ferrite's standalone server and throughput
client binaries.

This is narrower than a full Tier 1 HTTP throughput gate. It covers one local
host, one model, one prompt, one generated token per request, and the legacy
completion endpoint. It does not prove multi-model Tier 1 throughput, long
generation throughput, chat throughput, streaming throughput, x86_64 behavior,
multi-process load, long-running steady state, or production SLOs.

## Environment

- Commit: `3acca52`
- Hardware: Apple M1 Pro
- CPU count: 8
- Memory: 17179869184 bytes
- OS: macOS 14.5 23F79
- Kernel: `Darwin Victors-MBP.localdomain 23.5.0 Darwin Kernel Version 23.5.0: Wed May  1 20:12:58 PDT 2024; root:xnu-10063.121.3~5/RELEASE_ARM64_T6000 arm64`
- Execution target: local macOS
- Build mode: Cargo release profile

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m`
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
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m \
  --bind 127.0.0.1:18082 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18082/health
```

Result:

```json
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18082 \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=1
elapsed_ms=531
requests_per_second=1.879955
```

## Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18082 \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=10
elapsed_ms=5095
requests_per_second=1.962582
```

## Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18082 \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=9
elapsed_ms=4518
requests_per_second=1.991703
```

After the benchmark, `lsof -nP -iTCP:18082 -sTCP:LISTEN` returned no listener.

## Interpretation

The release OpenAI-compatible legacy completion path can serve bounded
one-token SmolLM2-1.7B Q4_K_M HTTP requests at about 1.96 requests per second
on this local Apple M1 Pro host. The queued three-client shape measured about
1.99 requests per second because model execution remains serialized by the
single inference permit.

This adds release binary request-rate evidence for a second Tier 1 model
family. Broader HTTP throughput remains open: chat, streaming, longer
generations, multiple prompts, Qwen2.5-0.5B, x86_64, longer steady-state runs,
and memory sampling all need separate evidence before Tier 1 HTTP throughput
can be treated as complete.

## Current-Tree Rerun: 2026-06-30

This rerun repeats the same bounded release-build request shape on the current
tree after additional OpenAI-compatible endpoint hardening. It remains a narrow
one-host, one-model, one-prompt, one-token measurement and does not close the
broader Tier 1 HTTP throughput gate.

### Environment

- Commit: `6171c33`
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
Finished `release` profile [optimized] target(s) in 0.04s
```

### Server

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m \
  --bind 127.0.0.1:18082 \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 16 \
  --inference-wait-ms 30000
```

Readiness check:

```sh
curl -fsS http://127.0.0.1:18082/health
```

Result:

```json
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m"}
```

Warmup:

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18082 \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 1 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=1
elapsed_ms=626
requests_per_second=1.596935
```

### Sequential Result

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18082 \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 10 \
  --concurrency 1 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=10
elapsed_ms=5216
requests_per_second=1.917118
```

### Queued Result

Ferrite's server still uses one inference permit, so this is queued request
handling rather than parallel model execution.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:18082 \
  --model smollm2-1.7b-q4_k_m \
  --prompt 'hello world' \
  --requests 9 \
  --concurrency 3 \
  --max-tokens 1 \
  --api-key local-secret
```

Result:

```text
openai_http_completion_requests=9
elapsed_ms=4773
requests_per_second=1.885466
```

After the benchmark, `lsof -nP -iTCP:18082 -sTCP:LISTEN` returned no listener.

### Interpretation

The current tree preserves the earlier release-build result shape: bounded
one-token SmolLM2-1.7B Q4_K_M completion requests measured about 1.9 requests
per second on this local Apple M1 Pro host. This rerun strengthens the release
evidence for the legacy completion endpoint only; chat, streaming, longer
generations, additional Tier 1 models, x86_64 behavior, steady-state load, and
memory sampling remain separate proof obligations.
