# Tier 1 Qwen2.5-1.5B Q8_0 HTTP Chat Streaming 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-1.5B-Instruct Q8_0 serving two sequential 32-token streaming
`POST /v1/chat/completions` requests.

This is the first 32-token chat-completion SSE server memory sample for the
larger Qwen2.5 Tier 1 artifact. It complements the existing non-streaming
Qwen2.5-1.5B Q8_0 chat memory sample and existing streaming functional and
throughput evidence. It is not a leak test, a concurrency memory test,
long-running steady-state evidence, x86_64 evidence, or full Tier 1 memory
completion.

## Environment

- Commit before documentation: `f66c426`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.22s
```

## Model

- Model: Qwen2.5-1.5B-Instruct Q8_0 GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Server model ID: `qwen2.5-1.5b-q8_0-chatstream32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0-chatstream32 \
  --bind 127.0.0.1:18094 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0-chatstream32"}
```

Server PID:

```text
1943
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
{"model":"qwen2.5-1.5b-q8_0-chatstream32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32,"stream":true,"stream_options":{"include_usage":true}}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 121,344 | 124,256,256 |
| After first 32-token streaming chat completion | 1,637,440 | 1,676,738,560 |
| Two seconds idle after first streaming chat completion | 1,637,440 | 1,676,738,560 |
| After second 32-token streaming chat completion | 1,647,936 | 1,687,486,464 |
| Two seconds idle after second streaming chat completion | 1,647,936 | 1,687,486,464 |

Both requests returned HTTP `200`, emitted `[DONE]`, and included usage in the
stream.

| Request | HTTP | Time total | Response bytes | SSE event chunks | Done | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First streaming chat completion | 200 | 3.971055 s | 9,075 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |
| Second streaming chat completion | 200 | 3.479137 s | 9,075 | 33 | true | `length` | `assistant` | 68 | 8 | 32 | 40 |

After the benchmark, `lsof -nP -iTCP:18094 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token streaming chat completion increased current RSS from about
124 MB after health to about 1.68 GB after the request. The second identical
streaming request completed successfully and stayed in the same range, ending
at about 1.69 GB after the request and after a two-second idle sample.

The sample narrows the local OpenAI-compatible SSE chat memory gap for one
larger Qwen2.5 Tier 1 artifact. It also confirms that the local streaming path
emits `[DONE]` and usage data for this 32-token real-model chat shape.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, broader streaming prompt
behavior, x86_64 behavior, or broader Tier 1 memory posture.
