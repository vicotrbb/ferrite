# OpenAI Long-Chat Qwen 0.5B Generated-Context Windowing Probe

## Scope

This run tests the generated-context windowing theory with a benchmark-only
long-chat gate flag. It compares the current unwindowed generated assistant
carry-forward against a 128-character generated-context window on the same
local server process.

This is a latency measurement probe, not a server default policy change. It
does not prove conversation quality, token-exact windowing, x86_64 behavior,
longer token budgets, or steady-state leak freedom.

## Environment

- Date: 2026-07-02
- Commit: `3766d7d`
- Host: local macOS development machine
- Server port: `127.0.0.1:18181`
- Server PID for RSS sampling: `66016`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Server binary SHA256:
  `1c5ccbb758fd03ab2dee640f9346f342c693dff49094f148abef3fea6867b8d3`
- Long-chat gate binary SHA256:
  `6775eafed6979b602bc64646ec09b2e01d59124f3c46f53ad777b2b45a18c9bd`
- API key: `local-secret`
- Baseline raw log:
  `target/proof/qwen-0-5b-long-chat-windowing-baseline-128.log`
- Windowed raw log:
  `target/proof/qwen-0-5b-long-chat-windowing-128chars-128.log`

After both gates completed and the server was stopped, `lsof -nP -iTCP:18181
-sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --bind 127.0.0.1:18181 \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Baseline Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18181 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --rss-pid 66016 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

## Windowed Gate

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18181 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --rss-pid 66016 \
  --generated-context-max-chars 128 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

## Results

Both gates completed four streaming chat turns with generated assistant context
on turns 2-4, token-limit status, usage accounting, streaming token IDs, RSS
samples, and `long_chat_summary_run_complete=true`.

| Run | Turn | Context | Prompt tokens | Completion tokens | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 1 | seed | 47 | 128 | 1983 | 7807 | 16.522197 | 431325184 | 446365696 | 446365696 |
| baseline | 2 | generated | 158 | 128 | 7265 | 13860 | 9.306980 | 446365696 | 433061888 | 433061888 |
| baseline | 3 | generated | 158 | 128 | 7126 | 13791 | 9.353265 | 433061888 | 437501952 | 437501952 |
| baseline | 4 | generated | 158 | 128 | 6957 | 13572 | 9.504718 | 437501952 | 437551104 | 437551104 |
| window-128-chars | 1 | seed | 47 | 128 | 1992 | 8265 | 15.606614 | 436994048 | 434241536 | 433848320 |
| window-128-chars | 2 | generated | 51 | 128 | 2130 | 8015 | 16.094768 | 433848320 | 441991168 | 441974784 |
| window-128-chars | 3 | generated | 57 | 128 | 2432 | 8337 | 15.471756 | 441974784 | 441221120 | 441221120 |
| window-128-chars | 4 | generated | 59 | 128 | 2501 | 8400 | 15.356261 | 441221120 | 445415424 | 445300736 |

Generated-turn averages:

| Metric | Baseline | Window 128 chars | Change |
| --- | ---: | ---: | ---: |
| Prompt tokens | 158.00 | 55.67 | -64.77% |
| TTFT ms | 7116.00 | 2354.33 | -66.91% |
| Stream ms | 13741.00 | 8250.67 | -39.96% |
| Streaming tok/s | 9.388321 | 15.640928 | +66.60% |

The shared summary invariants held for both runs:

```text
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_run_complete=true
```

## Interpretation

The 128-character generated-context window materially reduced follow-up prompt
size and first-token latency for this small local model and 128-token budget.
The result supports continuing the windowing theory as a benchmark and design
track.

The result is not enough to make windowing a default serving policy. The current
flag is character-based and benchmark-only. A production policy should be
token-aware, explicit about client-visible history retention, and validated on
larger models, longer budgets, reconnect/error probes, and conversation-quality
checks.
