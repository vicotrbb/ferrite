# Tier 1 SmolLM2-1.7B Q4_K_M HTTP Chat Streaming Concurrent Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for two overlapping SmolLM2-1.7B-Instruct Q4_K_M 32-token streaming
`POST /v1/chat/completions` requests.

This is a two-client overlap shape, not a throughput benchmark. Ferrite's
single inference permit is expected to serialize actual model execution while
the second client waits inside the configured bounded wait window. It
complements the matching Qwen2.5 Q8_0 and Q6_K concurrent streaming memory
samples. It is not a leak test, high-concurrency evidence, x86_64 evidence,
broad prompt evidence, or full Tier 1 memory completion.

## Environment

- Commit before documentation: `47864fa`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.24s
```

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m-chatstream32-concurrent`

## Server

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m-chatstream32-concurrent \
  --bind 127.0.0.1:18099 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 60000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m-chatstream32-concurrent"}
```

Server PID:

```text
6810
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Request `stream`: `true`
- Request `stream_options.include_usage`: `true`
- Clients: 2 concurrent streaming chat clients
- Client start spacing: second client started 0.051518 seconds after first
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID
- RSS sample interval during probe: 0.25 seconds

Request body:

```json
{"model":"smollm2-1.7b-q4_k_m-chatstream32-concurrent","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 106,832 | 109,395,968 |
| Maximum during two-client probe | 1,439,376 | 1,473,921,024 |
| After both clients completed | 1,067,184 | 1,092,796,416 |
| Two seconds idle after both clients completed | 1,067,184 | 1,092,796,416 |

The max RSS sample occurred 6.811455 seconds after probe start. The probe took
14.849324 seconds wall-clock and collected 57 RSS samples.

Both requests returned HTTP `200`, emitted `[DONE]`, and included usage in the
stream.

| Client | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat client | 200 | 7.634702 s | 9,959 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |
| Second streaming chat client | 200 | 14.709220 s | 9,959 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |

The observed finish order was `first_then_second`, consistent with the second
request waiting behind the first request's inference permit.

After the benchmark, `lsof -nP -iTCP:18099 -sTCP:LISTEN` returned no listener.

## Interpretation

The two-client streaming chat probe peaked at about 1.47 GB RSS while both
clients were active and settled at about 1.09 GB after completion and a
two-second idle sample. Both clients completed with HTTP `200`, `[DONE]`, and
32 completion tokens.

The result narrows one concurrent OpenAI-compatible streaming chat memory gap
for SmolLM2-1.7B Q4_K_M and complements the matching Qwen2.5 Q8_0 and Q6_K
evidence. It does not prove high concurrency, queue fairness beyond this
two-client shape, leak freedom, x86_64 behavior, broader prompts, or full Tier
1 memory posture.
