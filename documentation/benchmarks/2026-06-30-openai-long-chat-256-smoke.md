# OpenAI Long-Chat 256-Token Execute Smoke

## Scope

This is a bounded local smoke test for the dedicated OpenAI long-chat gate. It
proves that the gate can execute repeated streaming chat requests against a live
Ferrite server, capture per-token streaming latency, capture usage and finish
reason summaries, and sample server RSS before and after each request.

This is not the full long-chat milestone. The full gate still needs the complete
256/512/1024-token matrix, explicit stop-triggered behavior, EOS behavior, and
client reconnect/error behavior.

## Environment

- Date: 2026-06-30
- Commit: `55ef15f`
- Host: local macOS development machine
- Server port: `127.0.0.1:18091`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18091 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

RSS PID used by the gate: `6407`.

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 256 \
  --turns 4 \
  --addr 127.0.0.1:18091 \
  --api-key local-secret \
  --rss-pid 6407 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

## Results

All four planned streaming chat scenarios completed.

| Turn | Completed | Finish | Token events | Total ms | Stream ms | Tok/s | TTFT ms | p50 ms | p95 ms | RSS before | RSS after | RSS idle |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | length | 256 | 17215 | 15130 | 16.918943 | 2095 | 49 | 60 | 162512896 | 419266560 | 419266560 |
| 2 | 1 | length | 256 | 17015 | 14934 | 17.141514 | 2109 | 49 | 60 | 419266560 | 425263104 | 425263104 |
| 3 | 1 | length | 256 | 16901 | 14819 | 17.275075 | 1988 | 49 | 60 | 425263104 | 409616384 | 409616384 |
| 4 | 1 | length | 256 | 16844 | 14760 | 17.344003 | 2021 | 50 | 59 | 409616384 | 424656896 | 421724160 |

Usage was stable across all turns:

- Prompt tokens: `47`
- Completion tokens: `256`
- Total tokens: `303`

After stopping the server, `lsof -nP -iTCP:18091 -sTCP:LISTEN` returned no
listener.

## Interpretation

The smoke confirms a real model can complete four repeated 256-token streaming
chat responses through the OpenAI-compatible HTTP server and the dedicated
long-chat gate. The gate now records the basic evidence needed for the next
proof milestone: finish reason, token count, time to first token, token latency
distribution, throughput, and RSS before/after/idle.

Remaining proof gaps:

- Run the full 256, 512, and 1024-token matrix.
- Add explicit stop-sequence-triggered gate scenarios.
- Distinguish EOS completion from length-limited completion in long responses.
- Exercise client reconnect and mid-stream error behavior.
- Repeat across the agreed model set, not only Qwen2.5-0.5B Q4_K_M.
