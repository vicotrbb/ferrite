# OpenAI Long-Chat Qwen 0.5B Generated-Context Token Windowing Probe

## Scope

This run validates the benchmark-only generated-context token window in the
OpenAI-compatible long-chat gate. The new flag keeps only the trailing
streaming content chunks from a generated assistant response before carrying
that assistant text into the next generated-context turn.

This is a harness and measurement slice. It does not change Ferrite's default
OpenAI-compatible serving policy, and it does not prove conversation quality,
x86_64 behavior, larger models, longer token budgets, or steady-state memory
behavior.

## Environment

- Date: 2026-07-02
- Commit: `f5f052a`
- Host: local macOS development machine
- Server port: `127.0.0.1:18183`
- Server PID for RSS sampling: `74095`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Server binary SHA256:
  `32d83cbdaa326733b837ec4bc158817d388e9fb691d58447ea5e5ad2e30e1ca0`
- Long-chat gate binary SHA256:
  `ce67ca62bf772ceb3d586dc728502129d118eb2050aa968920db09defd9a53e5`
- API key: `local-secret`
- Raw proof log:
  `target/proof/qwen-0-5b-long-chat-windowing-32tokens-128.log`

After the gate completed and the server was stopped, `lsof -nP -iTCP:18183
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --bind 127.0.0.1:18183 \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18183 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --rss-pid 74095 \
  --generated-context-max-tokens 32 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

The plan output included:

```text
long_chat_generated_context_max_tokens=32
```

## Results

The gate completed four streaming chat turns with generated assistant context
on turns 2-4, token-limit status, usage accounting, streaming token IDs, RSS
samples, and `long_chat_summary_run_complete=true`.

| Turn | Context | Prompt tokens | Completion tokens | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 128 | 2172 | 8526 | 15.129185 | 256000000 | 445956096 | 445956096 |
| 2 | generated | 62 | 128 | 3000 | 9243 | 13.955233 | 445956096 | 454950912 | 438747136 |
| 3 | generated | 61 | 128 | 3389 | 9707 | 13.289052 | 438747136 | 423673856 | 423673856 |
| 4 | generated | 62 | 128 | 4015 | 10285 | 12.541721 | 423673856 | 423919616 | 423919616 |

Generated-turn averages compared with prior local probes:

| Metric | Unwindowed | 128-char window | 32-token window |
| --- | ---: | ---: | ---: |
| Prompt tokens | 158.00 | 55.67 | 61.67 |
| TTFT ms | 7116.00 | 2354.33 | 3468.00 |
| Stream ms | 13741.00 | 8250.67 | 9745.00 |
| Streaming tok/s | 9.388321 | 15.640928 | 13.262002 |

The token-window run recorded:

```text
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_run_complete=true
```

## Interpretation

The 32-token generated-context window materially reduced prompt size and
first-token latency versus the unwindowed baseline while preserving the
long-chat gate invariants. It was slower than the earlier 128-character window
on this prompt, but it is semantically safer for future experiments because it
tracks generated streaming chunks instead of raw character count.

This supports using token-window sweeps as the next generated-context
windowing experiment before considering any default serving policy.
