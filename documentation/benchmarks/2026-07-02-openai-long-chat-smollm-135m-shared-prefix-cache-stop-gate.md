# Benchmark: SmolLM2 135M Shared-Prefix Cache Stop Gate

Date: 2026-07-02

## Purpose

Verify stop-sequence behavior through Ferrite's OpenAI-compatible long-chat
gate while the experimental shared-prefix cache is enabled and required for
generated follow-up turns.

This is a stop-focused proof. It complements the 256/512/1024 length-limited
full matrix; it does not replace it.

## Environment

- Ferrite commit: `2a9ea419c50f2d3efe475b6b044053d345b0569f`
- Code commit under test: `0a3ecc7070339a1180e20606be9c1898a0f6874f`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
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
  --default-max-tokens 8 \
  --hard-max-tokens 32 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness checks passed:

```text
GET /health -> {"status":"ok","ready":true,"model":"smollm2-135m-q4_k_m"}
GET /v1/models -> smollm2-135m-q4_k_m
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
  --token-lengths 2 \
  --turns 4 \
  --probe-max-tokens 2 \
  --rss-pid <server-pid> \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world' \
  --stop 'user' \
  --expect-finish-reason stop \
  --prompt-cache-key long-chat:stop-prefix \
  --require-cached-follow-ups
```

The command exited `0`.

## Results

| Turn | Context | Prompt tokens | Cached prompt tokens | Completion tokens | Finish | TTFT ms | Decode tok/s | RSS before | RSS after |
| ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 1 | seed | 20 | 0 | 2 | stop | 605 | 30.798845 | 130285568 | 132120576 |
| 2 | generated | 18 | 8 | 2 | stop | 310 | 31.015526 | 132120576 | 132710400 |
| 3 | generated | 18 | 18 | 2 | stop | 20 | 31.695261 | 132694016 | 134201344 |
| 4 | generated | 18 | 18 | 2 | stop | 20 | 32.147966 | 134201344 | 134283264 |

Summary:

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=false
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=3
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_streaming_token_ids_required=false
long_chat_summary_all_streaming_token_id_summaries_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=false
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

Probe summary:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=2
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=2
```

## Interpretation

This proves the shared-prefix cache can coexist with stop-sequence handling in
the OpenAI-compatible streaming long-chat path on a small real local model. All
generated follow-up turns used generated assistant context, reported
`finish_reason=stop`, and had nonzero cached prompt tokens.

The gate correctly did not require token IDs for this stop-focused run:
`long_chat_summary_streaming_token_ids_required=false`. The generated stop
sequence is trimmed from visible stream content, so visible content chunks did
not carry token IDs.

## Limits

This does not prove:

- tokenizer EOS behavior without an explicit OpenAI stop sequence;
- stop behavior on larger Tier 1 models after the shared-prefix cache change;
- x86_64 stop behavior;
- long-running RSS stability;
- `llama-benchy` stop or shared-prefix behavior.

## Next Step

Repeat cached generated-context stop/EOS coverage for the required Tier 1
models and add a tokenizer-EOS-specific proof when the harness can isolate EOS
termination from explicit stop-sequence termination.
