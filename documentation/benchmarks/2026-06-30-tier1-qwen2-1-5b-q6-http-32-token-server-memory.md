# Tier 1 Qwen2.5-1.5B Q6_K HTTP 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-1.5B-Instruct Q6_K serving two sequential 32-token
`POST /v1/completions` requests.

This mirrors the Q8_0 HTTP 32-token memory sample so the local Qwen2.5-1.5B
Q8_0 and Q6_K server paths both have longer-generation HTTP memory evidence.
It is not a leak test, a concurrency memory test, long-running steady-state
evidence, x86_64 evidence, or full Tier 1 memory completion.

## Environment

- Commit before documentation: `2f432e2`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.21s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Server model ID: `qwen2.5-1.5b-q6_k-http32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k-http32 \
  --bind 127.0.0.1:18087 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k-http32"}
ready_after_attempt=1
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/completions`
- Prompt: `hello world`
- Request `max_tokens`: 32
- Requests: 2 sequential non-streaming legacy completions
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID

Request body:

```json
{"model":"qwen2.5-1.5b-q6_k-http32","prompt":"hello world","max_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 875,712 | 896,729,088 |
| After first 32-token completion | 1,489,632 | 1,525,383,168 |
| Two seconds idle after first completion | 1,489,616 | 1,525,366,784 |
| After second 32-token completion | 1,488,944 | 1,524,678,656 |
| Two seconds idle after second completion | 1,488,944 | 1,524,678,656 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Finish reason | Text length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| First completion | 200 | 9.025192 s | 613 | `length` | 134 | 2 | 32 | 34 |
| Second completion | 200 | 8.967781 s | 613 | `length` | 134 | 2 | 32 | 34 |

After the benchmark, `lsof -nP -iTCP:18087 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token HTTP completion increased current RSS from about 0.90 GB
after health to about 1.53 GB after the request. As with the Q8_0 sample, this
likely includes lazy page faulting of model weights and runtime pages, so it
should not be interpreted as request-only KV-cache growth.

The second identical 32-token request completed successfully and kept RSS in the
same range: about 1.52 GB immediately after the request and after a two-second
idle sample. This narrows the local OpenAI-compatible server memory gap for one
Qwen2.5-1.5B Q6_K longer generation shape.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, chat
memory, x86_64 behavior, or broader Tier 1 memory posture.
