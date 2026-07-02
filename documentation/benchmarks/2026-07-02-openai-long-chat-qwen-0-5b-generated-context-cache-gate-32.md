# Benchmark: Qwen 0.5B Generated-Context Cache Gate 32

Date: 2026-07-02

## Purpose

Run the first generated-context long-chat cache gate against Ferrite's
OpenAI-compatible server with `--experimental-prefix-cache`.

This is intentionally a small 32-token proof. It checks whether the current
runtime prefix cache can satisfy `--require-cached-follow-ups` when follow-up
turns use generated assistant context.

## Environment

- Ferrite commit: `1d56c3aaf2e8ecc5a9666642b235ca734700c3b0`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
- Server: local Ferrite server on `127.0.0.1:18080`
- Server binary SHA256:
  `652393f177907ba1a01e7e72f9dcd131c5701da694117b6f07477bfb9aebfa35`
- Long-chat gate binary SHA256:
  `a3802153eeb1d587b37189d0ae4429dd1275cf193b19f66f1ca58587af069202`

## Model

- Name: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 64 \
  --hard-max-tokens 128 \
  --experimental-prefix-cache
```

Server log:
`target/proof/qwen-0-5b-generated-context-cache-gate-32-server.log`

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18080 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 32 \
  --turns 4 \
  --prompt 'Write a compact note about generated-context prefix caching.' \
  --assistant-context 'Generated-context cache validation starts with a seed assistant message.' \
  --follow-up 'Continue the note and preserve the prior context.' \
  --prompt-cache-key ferrite:long-chat:generated-context-cache-gate-32 \
  --require-cached-follow-ups \
  --rss-pid <server-pid>
```

Raw log:
`target/proof/qwen-0-5b-generated-context-cache-gate-32.log`

Exit marker:
`target/proof/qwen-0-5b-generated-context-cache-gate-32.exit`

The process exited `0`, which means the gate tool completed. The actual proof
verdict is in the `long_chat_summary_*` fields.

## Results

| Turn | Context source | Prompt tokens | Cached prompt tokens | TTFT ms | Decode tok/s | RSS before | RSS after |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 39 | 0 | 1671 | 23.822915 | 306348032 | 446889984 |
| 2 | generated | 62 | 0 | 2637 | 22.249346 | 446889984 | 449740800 |
| 3 | generated | 62 | 0 | 2635 | 22.837549 | 449740800 | 452182016 |
| 4 | generated | 62 | 0 | 2755 | 20.373598 | 452182016 | 450789376 |

Summary:

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=false
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=0
long_chat_summary_uncached_generated_follow_up_turns=3
long_chat_summary_all_generated_follow_up_turns_cached=false
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_run_complete=false
```

## Interpretation

This run proves the current exact-prompt cache does not satisfy generated-context
long-chat prefix reuse. Follow-up turns correctly used generated assistant
context, but every generated follow-up reported `cached_prompt_tokens=0`.

That is expected from the current implementation: the runtime prefix-cache key
uses the full tokenized prompt identity. Generated-context turns change the
prompt, so exact prompt reuse misses even when the same explicit
`prompt_cache_key` namespace is supplied.

The gate behavior is useful: `--require-cached-follow-ups` converted the cache
misses into `long_chat_summary_run_complete=false` without hiding the completed
turn-level evidence.

## Limits

This does not prove:

- partial-prefix KV reuse;
- lower generated-context TTFT;
- behavior at 256/512/1024 generated-token lengths;
- stop/EOS behavior under cached generated context;
- reconnect/error behavior under cached generated context;
- long-running RSS stability.

## Next Step

Implement a token-prefix match layer that can restore the longest valid cached
prefix and evaluate only the suffix. Keep it behind the existing experimental
prefix-cache flag until the generated-context gate can complete with cached
follow-ups and bounded RSS.
