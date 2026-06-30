# Tier 1 SmolLM2-1.7B Q4_K_M HTTP Chat Streaming Three-Client Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for three overlapping SmolLM2-1.7B-Instruct Q4_K_M 32-token streaming
`POST /v1/chat/completions` requests.

This is a three-client overlap shape, not a throughput benchmark. Ferrite's
single inference permit is expected to serialize actual model execution while
the second and third clients wait inside the configured bounded wait window. It
extends the existing two-client SmolLM2-1.7B Q4_K_M concurrent streaming memory
sample. It is not a leak test, broad high-concurrency evidence, x86_64
evidence, broad prompt evidence, or full Tier 1 memory completion.

## Environment

- Commit before documentation: `e2ad03c`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.25s
```

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m-chatstream32-three-client`

## Server

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m-chatstream32-three-client \
  --bind 127.0.0.1:18102 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m-chatstream32-three-client"}
```

Server PID:

```text
10819
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Request `stream`: `true`
- Request `stream_options.include_usage`: `true`
- Clients: 3 concurrent streaming chat clients
- Client start spacing: second client started 0.052781 seconds after first
- Client start spacing: third client started 0.105795 seconds after first
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- RSS sample interval during probe: 0.25 seconds

Request body:

```json
{"model":"smollm2-1.7b-q4_k_m-chatstream32-three-client","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 422,976 | 433,127,424 |
| Maximum during three-client probe | 1,448,720 | 1,483,489,280 |
| After all clients completed | 1,071,232 | 1,096,941,568 |
| Two seconds idle after all clients completed | 1,071,232 | 1,096,941,568 |

The max RSS sample occurred 6.211098 seconds after probe start. The probe took
21.859903 seconds wall-clock and collected 83 RSS samples.

All three requests returned HTTP `200`, emitted `[DONE]`, and included usage in
the stream.

| Client | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat client | 200 | 7.395955 s | 10,029 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |
| Second streaming chat client | 200 | 14.485997 s | 10,029 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |
| Third streaming chat client | 200 | 21.502042 s | 10,029 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |

The observed finish order was `first_then_second_then_third`, consistent with
the second and third requests waiting behind the single inference permit.

After the benchmark, `lsof -nP -iTCP:18102 -sTCP:LISTEN` returned no listener.

## Interpretation

The three-client streaming chat probe peaked at about 1.48 GB RSS while clients
were active and settled at about 1.10 GB after completion and a two-second idle
sample. All clients completed with HTTP `200`, `[DONE]`, and 32 completion
tokens.

The result narrows one higher-concurrency OpenAI-compatible streaming chat
memory gap for SmolLM2-1.7B Q4_K_M. It does not prove broad high concurrency,
queue fairness beyond this three-client shape, leak freedom, x86_64 behavior,
broader prompts, or full Tier 1 memory posture.
