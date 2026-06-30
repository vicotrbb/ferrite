# Tier 1 SmolLM2-1.7B Q4_K_M HTTP Chat 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for SmolLM2-1.7B-Instruct Q4_K_M serving two sequential 32-token
`POST /v1/chat/completions` requests.

This extends 32-token chat-completion server memory evidence beyond the Qwen2.5
family to the current Tier 1 SmolLM2 artifact. It complements the existing
SmolLM2-1.7B Q4_K_M 32-token legacy completion server memory sample. It is not
a leak test, a concurrency memory test, streaming memory evidence,
long-running steady-state evidence, x86_64 evidence, or full Tier 1 memory
completion.

## Environment

- Commit before documentation: `d23291a`
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

- Model: SmolLM2-1.7B-Instruct Q4_K_M GGUF
- Path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Server model ID: `smollm2-1.7b-q4_k_m-chat32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id smollm2-1.7b-q4_k_m-chat32 \
  --bind 127.0.0.1:18093 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"smollm2-1.7b-q4_k_m-chat32"}
```

Server PID:

```text
1124
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
{"model":"smollm2-1.7b-q4_k_m-chat32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 636,288 | 651,558,912 |
| After first 32-token chat completion | 1,059,952 | 1,085,390,848 |
| Two seconds idle after first chat completion | 1,059,952 | 1,085,390,848 |
| After second 32-token chat completion | 1,064,784 | 1,090,338,816 |
| Two seconds idle after second chat completion | 1,064,784 | 1,090,338,816 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Object | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First chat completion | 200 | 7.479591 s | 613 | `chat.completion` | `length` | `assistant` | 60 | 9 | 32 | 41 |
| Second chat completion | 200 | 7.847837 s | 613 | `chat.completion` | `length` | `assistant` | 60 | 9 | 32 | 41 |

After the benchmark, `lsof -nP -iTCP:18093 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token chat completion increased current RSS from about 652 MB
after health to about 1.09 GB after the request. The second identical request
completed successfully and ended in the same range at about 1.09 GB after the
request and after a two-second idle sample.

This narrows one local OpenAI-compatible chat server memory gap for the
SmolLM2-1.7B Tier 1 artifact and extends 32-token chat memory evidence beyond
the Qwen2.5 family.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, broader
chat prompt behavior, x86_64 behavior, or broader Tier 1 memory posture.
