# OpenAI Long-Chat Qwen 1.5B Q8 Integrated Stop Gate

## Scope

This run exercises the long-chat gate's explicit stop behavior through the
integrated summary path for Qwen2.5 1.5B Q8_0. It combines explicit OpenAI
`stop` sequence handling, expected `finish_reason=stop`, four repeated turns,
RSS sampling, terminal stop timing, request-level error recovery, client
disconnect recovery, and final `long_chat_summary_*` output.

This is a single-model stop proof. It does not prove EOS behavior and does not
close the full Tier 1 long-chat gate across all required models.

## Environment

- Date: 2026-06-30
- Commit: `16f8a7e`
- Host: local macOS development machine
- Server port: `127.0.0.1:18101`
- Server PID for RSS sampling: `18331`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18101 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-1.5B-Instruct-Q8_0 \
  --token-lengths 1 \
  --turns 4 \
  --addr 127.0.0.1:18101 \
  --api-key local-secret \
  --rss-pid 18331 \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world' \
  --stop '你' \
  --expect-finish-reason stop
```

## Results

Probe results:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
```

All four stop scenarios completed with `finish_reason=stop`. Usage counted one
generated token, and terminal stop timing was present for every turn.

| Turn | Max tokens | Completed | Finish | Total ms | Stream events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 1 | stop | 9135 | 1 | 7065 | 7065 | 0.141531 | 1676574720 | 1670004736 | 1670004736 |
| 2 | 1 | 1 | stop | 9357 | 1 | 7303 | 7303 | 0.136912 | 1670004736 | 1673641984 | 1673641984 |
| 3 | 1 | 1 | stop | 10427 | 1 | 8378 | 8378 | 0.119356 | 1673641984 | 1667383296 | 1666957312 |
| 4 | 1 | 1 | stop | 4902 | 1 | 2884 | 2884 | 0.346728 | 1666957312 | 1679212544 | 1679212544 |

Usage was stable for every turn:

- prompt tokens `18`;
- completion tokens `1`;
- total tokens `19`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_run_complete=true
```

After stopping the server, `lsof -nP -iTCP:18101 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has integrated stop-summary evidence for the Qwen2.5 1.5B Q8_0
OpenAI-compatible streaming chat path. The run proves the larger Qwen Q8 model
can satisfy the same stop, reconnect/error, RSS, and terminal timing summary
shape already proven for Qwen2.5 0.5B Q4_K_M.

Remaining proof gaps:

- Repeat integrated stop-summary runs for Qwen2.5 1.5B Q6_K and SmolLM2 1.7B
  Q4_K_M.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Repeat integrated full-length error/disconnect summary runs for the larger
  required Tier 1 models.
