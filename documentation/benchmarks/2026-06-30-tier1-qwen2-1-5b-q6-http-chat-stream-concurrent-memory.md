# Tier 1 Qwen2.5-1.5B Q6_K HTTP Chat Streaming Concurrent Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for two overlapping Qwen2.5-1.5B-Instruct Q6_K 32-token streaming
`POST /v1/chat/completions` requests.

This is a two-client overlap shape, not a throughput benchmark. Ferrite's
single inference permit is expected to serialize actual model execution while
the second client waits inside the configured bounded wait window. It
complements the matching Qwen2.5-1.5B Q8_0 concurrent streaming memory sample.
It is not a leak test, high-concurrency evidence, x86_64 evidence, broad prompt
evidence, or full Tier 1 memory completion.

## Environment

- Commit before documentation: `4753b86`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.26s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Server model ID: `qwen2.5-1.5b-q6_k-chatstream32-concurrent`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k-chatstream32-concurrent \
  --bind 127.0.0.1:18098 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k-chatstream32-concurrent"}
```

Server PID:

```text
5741
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Request `stream`: `true`
- Request `stream_options.include_usage`: `true`
- Clients: 2 concurrent streaming chat clients
- Client start spacing: second client started 0.052346 seconds after first
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- RSS sample interval during probe: 0.25 seconds

Request body:

```json
{"model":"qwen2.5-1.5b-q6_k-chatstream32-concurrent","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 1,728 | 1,769,472 |
| Maximum during two-client probe | 2,371,024 | 2,427,928,576 |
| After both clients completed | 1,456,240 | 1,491,189,760 |
| Two seconds idle after both clients completed | 1,456,240 | 1,491,189,760 |

The max RSS sample occurred 17.843684 seconds after probe start. The probe took
24.355383 seconds wall-clock and collected 93 RSS samples.

Both requests returned HTTP `200`, emitted `[DONE]`, and included usage in the
stream.

| Client | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat client | 200 | 11.873789 s | 9,719 | 34 | true | `length` | `assistant` | 77 | 8 | 32 | 40 |
| Second streaming chat client | 200 | 24.049487 s | 9,719 | 34 | true | `length` | `assistant` | 77 | 8 | 32 | 40 |

The observed finish order was `first_then_second`, consistent with the second
request waiting behind the first request's inference permit.

After the benchmark, `lsof -nP -iTCP:18098 -sTCP:LISTEN` returned no listener.

## Interpretation

The two-client streaming chat probe peaked at about 2.43 GB RSS while both
clients were active and settled at about 1.49 GB after completion and a
two-second idle sample. Both clients completed with HTTP `200`, `[DONE]`, and
32 completion tokens.

The result narrows one concurrent OpenAI-compatible streaming chat memory gap
for Qwen2.5-1.5B Q6_K and complements the matching Q8_0 evidence. It does not
prove high concurrency, queue fairness beyond this two-client shape, leak
freedom, x86_64 behavior, broader prompts, or full Tier 1 memory posture.
