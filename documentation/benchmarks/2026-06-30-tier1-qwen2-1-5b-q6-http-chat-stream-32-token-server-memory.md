# Tier 1 Qwen2.5-1.5B Q6_K HTTP Chat Streaming 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-1.5B-Instruct Q6_K serving two sequential 32-token streaming
`POST /v1/chat/completions` requests.

This complements the Qwen2.5-1.5B Q8_0 32-token streaming chat memory sample
with the matching Q6_K quantization. It is not a leak test, a concurrency
memory test, long-running steady-state evidence, x86_64 evidence, or full Tier
1 memory completion.

## Environment

- Commit before documentation: `c5c0550`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.29s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Server model ID: `qwen2.5-1.5b-q6_k-chatstream32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k-chatstream32 \
  --bind 127.0.0.1:18095 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k-chatstream32"}
```

Server PID:

```text
2753
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Request `stream`: `true`
- Request `stream_options.include_usage`: `true`
- Requests: 2 sequential streaming chat completions
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID

Request body:

```json
{"model":"qwen2.5-1.5b-q6_k-chatstream32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 894,448 | 915,914,752 |
| After first 32-token streaming chat completion | 1,470,240 | 1,505,525,760 |
| Two seconds idle after first streaming chat completion | 1,470,240 | 1,505,525,760 |
| After second 32-token streaming chat completion | 1,455,744 | 1,490,681,856 |
| Two seconds idle after second streaming chat completion | 1,455,744 | 1,490,681,856 |

Both requests returned HTTP `200`, emitted `[DONE]`, and included usage in the
stream.

| Request | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat completion | 200 | 11.160442 s | 9,345 | 34 | true | `length` | `assistant` | 77 | 8 | 32 | 40 |
| Second streaming chat completion | 200 | 11.440360 s | 9,345 | 34 | true | `length` | `assistant` | 77 | 8 | 32 | 40 |

After the benchmark, `lsof -nP -iTCP:18095 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token streaming chat completion increased current RSS from about
916 MB after health to about 1.51 GB after the request. The second identical
streaming request completed successfully and ended slightly lower, about 1.49
GB after the request and after a two-second idle sample.

The sample gives Qwen2.5-1.5B Q6_K the same bounded local
OpenAI-compatible SSE chat memory shape already recorded for Qwen2.5-1.5B
Q8_0.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, broader streaming prompt
behavior, x86_64 behavior, or broader Tier 1 memory posture.
