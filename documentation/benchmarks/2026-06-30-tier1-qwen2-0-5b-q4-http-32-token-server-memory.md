# Tier 1 Qwen2.5-0.5B Q4_K_M HTTP 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-0.5B-Instruct Q4_K_M serving two sequential 32-token
`POST /v1/completions` requests.

This broadens the local 32-token HTTP memory evidence beyond the Qwen2.5-1.5B
Q8_0 and Q6_K server probes. It is not a leak test, a concurrency memory test,
long-running steady-state evidence, x86_64 evidence, or full Tier 1 memory
completion.

## Environment

- Commit before documentation: `74d8e40`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.16s
```

## Model

- Model: Qwen2.5-0.5B-Instruct Q4_K_M GGUF
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Server model ID: `qwen2.5-0.5b-q4_k_m-http32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m-http32 \
  --bind 127.0.0.1:18088 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m-http32"}
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
{"model":"qwen2.5-0.5b-q4_k_m-http32","prompt":"hello world","max_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 420,784 | 430,882,816 |
| After first 32-token completion | 433,616 | 444,022,784 |
| Two seconds idle after first completion | 433,600 | 444,006,400 |
| After second 32-token completion | 442,688 | 453,312,512 |
| Two seconds idle after second completion | 442,688 | 453,312,512 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Finish reason | Text length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| First completion | 200 | 1.484633 s | 613 | `length` | 132 | 2 | 32 | 34 |
| Second completion | 200 | 1.504314 s | 613 | `length` | 132 | 2 | 32 | 34 |

After the benchmark, `lsof -nP -iTCP:18088 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token HTTP completion increased current RSS from about 431 MB
after health to about 444 MB after the request. After a two-second idle sample,
RSS stayed in the same range.

The second identical 32-token request completed successfully and ended at about
453 MB, with the same RSS after a two-second idle sample. This narrows the
local OpenAI-compatible server memory gap for the smaller Qwen2.5 Tier 1
artifact.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, chat
memory, x86_64 behavior, or broader Tier 1 memory posture.
