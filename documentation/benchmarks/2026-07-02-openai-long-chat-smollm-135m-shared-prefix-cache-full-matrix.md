# Benchmark: SmolLM2 135M Shared-Prefix Cache Full Matrix

Date: 2026-07-02

## Purpose

Run the dedicated generated-context long-chat gate at 256, 512, and 1024
streaming response tokens after adding shared-prefix KV reuse.

This run exercises:

- 256, 512, and 1024-token streaming responses;
- four-turn generated-context conversations;
- RSS samples before, after, and after idle;
- latency per token and time-to-first-token summaries;
- unauthorized request recovery;
- client disconnect/reconnect behavior;
- OpenAI-compatible streaming token-id summaries;
- `--require-cached-follow-ups`.

## Environment

- Ferrite commit: `5f92f4e69edc2b944fcf544fb97913ac162183df`
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
  --default-max-tokens 64 \
  --hard-max-tokens 1024 \
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
  --token-lengths 256,512,1024 \
  --turns 4 \
  --probe-max-tokens 1024 \
  --rss-pid <server-pid> \
  --prompt-cache-key long-chat:prefix-full \
  --require-cached-follow-ups
```

The command exited `0`.

## Results

| Turn | Max tokens | Context | Prompt tokens | Cached prompt tokens | TTFT ms | Decode tok/s | RSS before | RSS after |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | seed | 48 | 0 | 1399 | 26.283714 | 133578752 | 137904128 |
| 1 | 512 | seed | 48 | 48 | 24 | 22.779998 | 137904128 | 134676480 |
| 1 | 1024 | seed | 48 | 48 | 24 | 18.079179 | 134676480 | 132661248 |
| 2 | 256 | generated | 282 | 14 | 9238 | 20.532027 | 132661248 | 156041216 |
| 2 | 512 | generated | 531 | 263 | 12036 | 15.440170 | 156041216 | 158433280 |
| 2 | 1024 | generated | 1028 | 512 | 31654 | 9.916060 | 158433280 | 132218880 |
| 3 | 256 | generated | 289 | 15 | 9549 | 20.471714 | 132218880 | 162185216 |
| 3 | 512 | generated | 545 | 526 | 1069 | 15.443163 | 161267712 | 167395328 |
| 3 | 1024 | generated | 1057 | 15 | 54674 | 9.430912 | 167395328 | 134316032 |
| 4 | 256 | generated | 276 | 33 | 8613 | 20.541765 | 134316032 | 163577856 |
| 4 | 512 | generated | 545 | 34 | 20990 | 14.951138 | 163577856 | 131792896 |
| 4 | 1024 | generated | 1004 | 33 | 50125 | 9.632236 | 131792896 | 130170880 |

Summary:

```text
long_chat_summary_planned_scenarios=12
long_chat_summary_completed_scenarios=12
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_prompt_cache_key_present=true
long_chat_summary_cached_follow_ups_required=true
long_chat_summary_any_cached_prompt_tokens=true
long_chat_summary_generated_follow_up_turns=9
long_chat_summary_cached_generated_follow_up_turns=9
long_chat_summary_uncached_generated_follow_up_turns=0
long_chat_summary_all_generated_follow_up_turns_cached=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_streaming_token_ids_required=true
long_chat_summary_all_streaming_token_id_summaries_present=true
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

Probe summary:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=1024
```

## Interpretation

This is the first local full-matrix proof that the shared-prefix cache can
satisfy the generated-context cached-follow-up gate at 256, 512, and 1024
streaming response tokens on a real model.

Every generated follow-up turn reported nonzero `cached_prompt_tokens`.
The matrix intentionally reuses one cache namespace across token budgets, so
some seed scenarios also reused the 48-token seed prompt from earlier rows.

The mixed generated-context rows show an important optimization boundary:
cached prompt-token counts depend on how much generated assistant content is
shared between the previous cached prompt and the current request. Some rows
hit large prefixes, while others only share the stable leading prompt segment.

## Limits

This does not prove:

- larger Tier 1 model behavior;
- x86_64 behavior;
- stop/EOS behavior under cached generated context;
- long-running RSS stability;
- leak freedom;
- `llama-benchy` shared-prefix benchmark results.

## Next Step

Repeat this full gate for the required Tier 1 models, add a stop/EOS-specific
variant, and then run a bounded `llama-benchy` shared-prefix comparison for
throughput and latency trends.
