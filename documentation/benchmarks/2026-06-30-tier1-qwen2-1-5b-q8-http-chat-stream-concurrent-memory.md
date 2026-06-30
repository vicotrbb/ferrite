# Tier 1 Qwen2.5-1.5B Q8_0 HTTP Chat Streaming Concurrent Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for two overlapping Qwen2.5-1.5B-Instruct Q8_0 32-token streaming
`POST /v1/chat/completions` requests.

This is a two-client overlap shape, not a throughput benchmark. Ferrite's
single inference permit is expected to serialize actual model execution while
the second client waits inside the configured bounded wait window. It is not a
leak test, high-concurrency evidence, x86_64 evidence, broad prompt evidence,
or full Tier 1 memory completion.

## Environment

- Commit before documentation: `01ab533`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.27s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q8_0 GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Server model ID: `qwen2.5-1.5b-q8_0-chatstream32-concurrent`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0-chatstream32-concurrent \
  --bind 127.0.0.1:18097 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0-chatstream32-concurrent"}
```

Server PID:

```text
4758
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Request `stream`: `true`
- Request `stream_options.include_usage`: `true`
- Clients: 2 concurrent streaming chat clients
- Client start spacing: second client started 0.059216 seconds after first
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- RSS sample interval during probe: 0.25 seconds

Request body:

```json
{"model":"qwen2.5-1.5b-q8_0-chatstream32-concurrent","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 209,696 | 214,728,704 |
| Maximum during two-client probe | 1,653,312 | 1,692,991,488 |
| After both clients completed | 1,630,416 | 1,669,545,984 |
| Two seconds idle after both clients completed | 1,630,080 | 1,669,201,920 |

The max RSS sample occurred 3.981771 seconds after probe start. The probe took
8.188751 seconds wall-clock and collected 31 RSS samples.

Both requests returned HTTP `200`, emitted `[DONE]`, and included usage in the
stream.

| Client | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat client | 200 | 3.937988 s | 9,438 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |
| Second streaming chat client | 200 | 8.041564 s | 9,438 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |

The observed finish order was `first_then_second`, consistent with the second
request waiting behind the first request's inference permit.

After the benchmark, `lsof -nP -iTCP:18097 -sTCP:LISTEN` returned no listener.

## Interpretation

The two-client streaming chat probe peaked at about 1.69 GB RSS while both
clients were active and settled at about 1.67 GB after completion and a
two-second idle sample. Both clients completed with HTTP `200`, `[DONE]`, and
32 completion tokens.

The result narrows one concurrent OpenAI-compatible streaming chat memory gap
for Qwen2.5-1.5B Q8_0. It does not prove high concurrency, queue fairness
beyond this two-client shape, leak freedom, x86_64 behavior, broader prompts,
or full Tier 1 memory posture.
