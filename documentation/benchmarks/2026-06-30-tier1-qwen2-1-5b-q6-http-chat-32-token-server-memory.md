# Tier 1 Qwen2.5-1.5B Q6_K HTTP Chat 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-1.5B-Instruct Q6_K serving two sequential 32-token
`POST /v1/chat/completions` requests.

This complements the Qwen2.5-1.5B Q8_0 32-token chat memory sample and the
existing Qwen2.5-1.5B Q6_K 32-token legacy completion memory sample. It is not
a leak test, a concurrency memory test, streaming memory evidence,
long-running steady-state evidence, x86_64 evidence, or full Tier 1 memory
completion.

## Environment

- Commit before documentation: `4b797a8`
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

- Model: Qwen2.5-1.5B-Instruct Q6_K GGUF
- Path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Server model ID: `qwen2.5-1.5b-q6_k-chat32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id qwen2.5-1.5b-q6_k-chat32 \
  --bind 127.0.0.1:18092 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q6_k-chat32"}
```

Server PID:

```text
121
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
{"model":"qwen2.5-1.5b-q6_k-chat32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 909,312 | 931,135,488 |
| After first 32-token chat completion | 1,468,608 | 1,503,854,592 |
| Two seconds idle after first chat completion | 1,468,608 | 1,503,854,592 |
| After second 32-token chat completion | 1,464,272 | 1,499,414,528 |
| Two seconds idle after second chat completion | 1,464,272 | 1,499,414,528 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Object | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First chat completion | 200 | 11.697414 s | 716 | `chat.completion` | `length` | `assistant` | 77 | 8 | 32 | 40 |
| Second chat completion | 200 | 11.178414 s | 716 | `chat.completion` | `length` | `assistant` | 77 | 8 | 32 | 40 |

After the benchmark, `lsof -nP -iTCP:18092 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token chat completion increased current RSS from about 931 MB
after health to about 1.50 GB after the request. The second identical request
completed successfully and remained in the same range, ending at about 1.50 GB
immediately after the request and after a two-second idle sample.

This narrows the local OpenAI-compatible chat server memory gap for the
Qwen2.5-1.5B Q6_K Tier 1 artifact and gives the two larger local Qwen2.5
quantizations matching 32-token chat memory samples.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, broader
chat prompt behavior, x86_64 behavior, or broader Tier 1 memory posture.
