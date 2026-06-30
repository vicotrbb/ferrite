# Tier 1 SmolLM2-1.7B Q4_K_M HTTP Chat Streaming 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for SmolLM2-1.7B-Instruct Q4_K_M serving two sequential 32-token streaming
`POST /v1/chat/completions` requests.

This extends 32-token chat-completion SSE server memory evidence beyond the
Qwen2.5 family to the current Tier 1 SmolLM2 artifact. It complements the
existing SmolLM2-1.7B Q4_K_M non-streaming chat memory sample and existing
streaming functional and throughput evidence. It is not a leak test, a
concurrency memory test, long-running steady-state evidence, x86_64 evidence,
or full Tier 1 memory completion.

## Environment

- Commit before documentation: `3a500fa`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.28s
```

## Model

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m-chatstream32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m-chatstream32 \
  --bind 127.0.0.1:18096 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m-chatstream32"}
```

Server PID:

```text
3615
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
{"model":"smollm2-1.7b-q4_k_m-chatstream32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 1,904 | 1,949,696 |
| After first 32-token streaming chat completion | 1,063,344 | 1,088,864,256 |
| Two seconds idle after first streaming chat completion | 1,063,344 | 1,088,864,256 |
| After second 32-token streaming chat completion | 1,068,832 | 1,094,483,968 |
| Two seconds idle after second streaming chat completion | 1,068,832 | 1,094,483,968 |

Both requests returned HTTP `200`, emitted `[DONE]`, and included usage in the
stream.

| Request | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat completion | 200 | 7.667215 s | 9,574 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |
| Second streaming chat completion | 200 | 7.381934 s | 9,574 | 35 | true | `length` | `assistant` | 60 | 9 | 32 | 41 |

After the benchmark, `lsof -nP -iTCP:18096 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token streaming chat completion increased current RSS from about
1.9 MB after health to about 1.09 GB after the request. The low post-health RSS
is consistent with memory-mapped model pages not being resident until first
inference. The second identical streaming request completed successfully and
ended in the same range, about 1.09 GB after the request and after a two-second
idle sample.

The sample narrows the local OpenAI-compatible SSE chat memory gap for the
SmolLM2-1.7B Tier 1 artifact and gives the current Qwen2.5 and SmolLM2 Tier 1
families bounded 32-token streaming chat memory samples.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, broader streaming prompt
behavior, x86_64 behavior, or broader Tier 1 memory posture.
