# Benchmark: SmolLM2 135M Shared-Prefix Cache Gate 32

Date: 2026-07-02

## Purpose

Verify that Ferrite's experimental OpenAI-compatible prefix cache can satisfy a
generated-context long-chat gate after adding shared token-prefix reuse.

This is a small 32-token proof against a real local model. It is not the full
256/512/1024-token milestone.

## Environment

- Ferrite commit: `0a3ecc7070339a1180e20606be9c1898a0f6874f`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
- Server: local Ferrite server on `127.0.0.1:18080`
- Server binary SHA256:
  `2528590df4e81a3e0c415ce3f903826055a1a12272ddcf8d960ef48519b244ef`
- Long-chat gate binary SHA256:
  `428b41c225b61e36441ec8c917fd902c561d51bd6f99379f2689ab57f92d693d`

## Model

- Name: `SmolLM2-135M-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Served model id: `smollm2-135m-q4_k_m`
- Model SHA256:
  `2e8040ceae7815abe0dcb3540b9995eaa1fa0d2ca9e797d0a635ae4433c68c2d`

## Server Command

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf \
  --model-id smollm2-135m-q4_k_m \
  --bind 127.0.0.1:18080 \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 128 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

## Gate Command

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18080 \
  --api-key local-secret \
  --models smollm2-135m-q4_k_m \
  --token-lengths 32 \
  --turns 4 \
  --probe-max-tokens 32 \
  --rss-pid <server-pid> \
  --prompt-cache-key long-chat:prefix \
  --require-cached-follow-ups
```

The command exited `0`.

## Results

| Turn | Context source | Prompt tokens | Cached prompt tokens | TTFT ms | Decode tok/s | RSS before | RSS after |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 48 | 0 | 1416 | 29.790906 | 240418816 | 241991680 |
| 2 | generated | 65 | 14 | 1537 | 30.420079 | 241991680 | 248070144 |
| 3 | generated | 65 | 65 | 24 | 30.772779 | 248070144 | 251625472 |
| 4 | generated | 65 | 65 | 24 | 29.736313 | 251625472 | 254279680 |

Summary:

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Interpretation

This proves the shared-prefix cache path can satisfy the current strict
generated-context cache gate shape on a small real local model. Turn 2 reused
14 prompt tokens from a divergent earlier prompt; turns 3 and 4 reused the full
65-token prompt.

The result also preserved the OpenAI-compatible streaming proof fields:
finish reasons were present, usage accounting was valid, token-id chunk
summaries were complete, RSS samples were present, the unauthorized request
probe recovered, and the disconnect/reconnect probe started a fresh generation.

## Limits

This does not prove:

- the full 256/512/1024-token long-chat milestone;
- larger Tier 1 models;
- x86_64 behavior;
- stop/EOS behavior under cached generated context;
- long-running RSS stability;
- `llama-benchy` prefix-cache mode against the shared-prefix implementation.

## Next Step

Run the dedicated long-chat gate at 256, 512, and 1024 generated tokens with
`--require-cached-follow-ups`, RSS sampling, stop/EOS variants, and
reconnect/error probes. Then compare Ferrite's gate output with a bounded
`llama-benchy` prefix-cache run.
