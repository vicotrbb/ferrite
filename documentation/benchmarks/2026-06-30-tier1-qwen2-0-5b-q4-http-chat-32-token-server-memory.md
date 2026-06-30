# Tier 1 Qwen2.5-0.5B Q4_K_M HTTP Chat 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-0.5B-Instruct Q4_K_M serving two sequential 32-token
`POST /v1/chat/completions` requests.

This is the first local 32-token chat-completion HTTP memory sample. It
complements the existing 32-token legacy completion memory samples, but it does
not prove broader chat memory posture, streaming memory, concurrent memory,
long-running steady-state behavior, x86_64 behavior, or full Tier 1 memory
completion.

## Environment

- Commit before documentation: `efbfdc2`
- Hardware: Apple M1 Pro
- CPU count: 8 physical / 8 logical
- Memory: 17179869184 bytes
- OS: macOS 14.5 / Darwin 23.5.0 arm64
- Build mode: Cargo release profile
- Build command: `cargo build --release -p ferrite-server`

Build result:

```text
Finished `release` profile [optimized] target(s) in 0.23s
```

## Model

- Model: Qwen2.5-0.5B-Instruct Q4_K_M GGUF
- Path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Server model ID: `qwen2.5-0.5b-q4_k_m-chat32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m-chat32 \
  --bind 127.0.0.1:18086 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m-chat32"}
ready_after_attempt=1
```

## Protocol

- Host: local macOS aarch64
- Route: `POST /v1/chat/completions`
- Message: user `hello world`
- Request `max_completion_tokens`: 32
- Requests: 2 sequential non-streaming chat completions
- RSS source: `ps -o rss= -p "$pid"`, sampled against the direct server PID

Request body:

```json
{"model":"qwen2.5-0.5b-q4_k_m-chat32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 421,232 | 431,341,568 |
| After first 32-token chat completion | 431,472 | 441,827,328 |
| Two seconds idle after first chat completion | 431,456 | 441,810,944 |
| After second 32-token chat completion | 433,760 | 444,170,240 |
| Two seconds idle after second chat completion | 433,760 | 444,170,240 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Object | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First chat completion | 200 | 1.749014 s | 703 | `chat.completion` | `length` | `assistant` | 52 | 8 | 32 | 40 |
| Second chat completion | 200 | 1.826347 s | 703 | `chat.completion` | `length` | `assistant` | 52 | 8 | 32 | 40 |

After the benchmark, `lsof -nP -iTCP:18086 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token chat completion increased current RSS from about 431 MB
after health to about 442 MB after the request. After a two-second idle sample,
RSS stayed in the same range.

The second identical 32-token chat request completed successfully and ended at
about 444 MB, with the same RSS after a two-second idle sample. This narrows
one local OpenAI-compatible chat server memory gap for the smaller Qwen2.5 Tier
1 artifact.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, broader
chat prompt behavior, x86_64 behavior, or broader Tier 1 memory posture.
