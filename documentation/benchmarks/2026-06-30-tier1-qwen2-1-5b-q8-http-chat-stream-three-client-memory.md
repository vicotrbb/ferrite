# Tier 1 Qwen2.5-1.5B Q8_0 HTTP Chat Streaming Three-Client Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for three overlapping Qwen2.5-1.5B-Instruct Q8_0 32-token streaming
`POST /v1/chat/completions` requests.

This is a three-client overlap shape, not a throughput benchmark. Ferrite's
single inference permit is expected to serialize actual model execution while
the second and third clients wait inside the configured bounded wait window. It
extends the existing two-client Qwen2.5-1.5B Q8_0 concurrent streaming memory
sample. It is not a leak test, broad high-concurrency evidence, x86_64
evidence, broad prompt evidence, or full Tier 1 memory completion.

## Environment

- Commit before documentation: `12229db`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.32s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q8_0 GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Server model ID: `qwen2.5-1.5b-q8_0-chatstream32-three-client`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0-chatstream32-three-client \
  --bind 127.0.0.1:18100 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0-chatstream32-three-client"}
```

Server PID:

```text
7784
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Request `stream`: `true`
- Request `stream_options.include_usage`: `true`
- Clients: 3 concurrent streaming chat clients
- Client start spacing: second client started 0.050723 seconds after first
- Client start spacing: third client started 0.103350 seconds after first
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- RSS sample interval during probe: 0.25 seconds

Request body:

```json
{"model":"qwen2.5-1.5b-q8_0-chatstream32-three-client","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 174,832 | 179,027,968 |
| Maximum during three-client probe | 1,654,208 | 1,693,908,992 |
| After all clients completed | 1,653,920 | 1,693,614,080 |
| Two seconds idle after all clients completed | 1,653,920 | 1,693,614,080 |

The max RSS sample occurred 8.444692 seconds after probe start. The probe took
11.282957 seconds wall-clock and collected 43 RSS samples.

All three requests returned HTTP `200`, emitted `[DONE]`, and included usage in
the stream.

| Client | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat client | 200 | 4.222967 s | 9,504 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |
| Second streaming chat client | 200 | 7.637356 s | 9,504 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |
| Third streaming chat client | 200 | 11.089721 s | 9,504 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |

The observed finish order was `first_then_second_then_third`, consistent with
the second and third requests waiting behind the single inference permit.

After the benchmark, `lsof -nP -iTCP:18100 -sTCP:LISTEN` returned no listener.

## Interpretation

The three-client streaming chat probe peaked at about 1.69 GB RSS while clients
were active and remained in the same range after completion and a two-second
idle sample. All clients completed with HTTP `200`, `[DONE]`, and 32 completion
tokens.

The result narrows one higher-concurrency OpenAI-compatible streaming chat
memory gap for Qwen2.5-1.5B Q8_0. It does not prove broad high concurrency,
queue fairness beyond this three-client shape, leak freedom, x86_64 behavior,
broader prompts, or full Tier 1 memory posture.
