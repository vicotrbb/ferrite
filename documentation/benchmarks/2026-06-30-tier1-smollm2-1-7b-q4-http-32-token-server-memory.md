# Tier 1 SmolLM2-1.7B Q4_K_M HTTP 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for SmolLM2-1.7B-Instruct Q4_K_M serving two sequential 32-token
`POST /v1/completions` requests.

This mirrors the existing Qwen2.5 32-token HTTP memory samples for the other
local Tier 1 HTTP artifacts. It is not a leak test, a concurrency memory test,
long-running steady-state evidence, x86_64 evidence, or full Tier 1 memory
completion.

## Environment

- Commit before documentation: `037d444`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.17s
```

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m-http32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m-http32 \
  --bind 127.0.0.1:18089 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m-http32"}
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
{"model":"smollm2-1.7b-q4_k_m-http32","prompt":"hello world","max_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 682,176 | 698,548,224 |
| After first 32-token completion | 1,065,664 | 1,091,239,936 |
| Two seconds idle after first completion | 1,065,648 | 1,091,223,552 |
| After second 32-token completion | 1,068,496 | 1,094,139,904 |
| Two seconds idle after second completion | 1,068,496 | 1,094,139,904 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Finish reason | Text length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| First completion | 200 | 5.744274 s | 607 | `length` | 123 | 2 | 32 | 34 |
| Second completion | 200 | 5.673172 s | 607 | `length` | 123 | 2 | 32 | 34 |

After the benchmark, `lsof -nP -iTCP:18089 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token HTTP completion increased current RSS from about 699 MB
after health to about 1.09 GB after the request. As with the larger Qwen2.5
samples, this likely includes lazy page faulting of model weights and runtime
pages, so it should not be interpreted as request-only KV-cache growth.

The second identical 32-token request completed successfully and stayed in the
same range: about 1.09 GB immediately after the request and after a two-second
idle sample. This narrows the local OpenAI-compatible server memory gap for the
current SmolLM2-1.7B Tier 1 HTTP path.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, chat
memory, x86_64 behavior, or broader Tier 1 memory posture.
