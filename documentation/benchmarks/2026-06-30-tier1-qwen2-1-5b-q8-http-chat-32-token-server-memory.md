# Tier 1 Qwen2.5-1.5B Q8_0 HTTP Chat 32-Token Server Memory

Date: 2026-06-30

## Scope

This benchmark records a bounded local OpenAI-compatible server memory sample
for Qwen2.5-1.5B-Instruct Q8_0 serving two sequential 32-token
`POST /v1/chat/completions` requests.

This expands the Qwen2.5-1.5B Q8_0 server memory evidence beyond the existing
32-token legacy completion sample and the smaller Qwen2.5-0.5B Q4_K_M
32-token chat sample. It is not a leak test, a concurrency memory test,
streaming memory evidence, long-running steady-state evidence, x86_64
evidence, or full Tier 1 memory completion.

## Environment

- Commit before documentation: `eb8a360`
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
- Server model ID: `qwen2.5-1.5b-q8_0-chat32`

## Server

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id qwen2.5-1.5b-q8_0-chat32 \
  --bind 127.0.0.1:18091 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
{"status":"ok","ready":true,"model":"qwen2.5-1.5b-q8_0-chat32"}
```

Server PID:

```text
99334
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
{"model":"qwen2.5-1.5b-q8_0-chat32","messages":[{"role":"user","content":"hello world"}],"max_completion_tokens":32}
```

## Results

| Point | RSS KiB | RSS bytes |
| --- | ---: | ---: |
| After health | 1,776 | 1,818,624 |
| After first 32-token chat completion | 1,638,496 | 1,677,819,904 |
| Two seconds idle after first chat completion | 1,638,496 | 1,677,819,904 |
| After second 32-token chat completion | 1,647,424 | 1,686,962,176 |
| Two seconds idle after second chat completion | 1,642,336 | 1,681,752,064 |

Both requests returned HTTP `200`.

| Request | HTTP | Time total | Response bytes | Object | Finish reason | Role | Content length | Prompt tokens | Completion tokens | Total tokens |
| --- | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: | ---: |
| First chat completion | 200 | 4.433113 s | 710 | `chat.completion` | `length` | `assistant` | 68 | 8 | 32 | 40 |
| Second chat completion | 200 | 3.838910 s | 710 | `chat.completion` | `length` | `assistant` | 68 | 8 | 32 | 40 |

After the benchmark, `lsof -nP -iTCP:18091 -sTCP:LISTEN` returned no listener.

## Interpretation

The first 32-token chat completion increased current RSS from about 1.8 MB
after health to about 1.68 GB after the request. The low post-health RSS is
consistent with memory-mapped model pages not being resident until the first
inference touches them, so it should not be interpreted as request-only
KV-cache growth.

The second identical 32-token chat request completed successfully and kept RSS
in the same range: about 1.69 GB immediately after the request and about 1.68
GB after a two-second idle sample. This narrows the local
OpenAI-compatible chat server memory gap for one larger Qwen2.5 Tier 1
artifact and complements the existing 32-token legacy completion memory sample.

The result remains bounded. It does not prove leak freedom, long-running
steady-state behavior, concurrent serving memory, streaming memory, broader
chat prompt behavior, x86_64 behavior, or broader Tier 1 memory posture.
