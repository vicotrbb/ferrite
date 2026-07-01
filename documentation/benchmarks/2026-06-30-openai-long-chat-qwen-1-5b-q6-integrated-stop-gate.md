# OpenAI Long-Chat Qwen 1.5B Q6 Integrated Stop Gate

## Scope

This run exercises the long-chat gate's explicit stop behavior through the
integrated summary path for Qwen2.5 1.5B Q6_K. It combines explicit OpenAI
`stop` sequence handling, expected `finish_reason=stop`, four repeated turns,
RSS sampling, terminal stop timing, request-level error recovery, client
disconnect recovery, and final `long_chat_summary_*` output.

This is a single-model stop proof. It does not prove EOS behavior and does not
close the full Tier 1 long-chat gate across all required models.

## Environment

- Date: 2026-06-30
- Commit: `c033630`
- Host: local macOS development machine
- Server port: `127.0.0.1:18102`
- Server PID for RSS sampling: `21173`
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18102 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-1.5B-Instruct-Q6_K \
  --token-lengths 1 \
  --turns 4 \
  --addr 127.0.0.1:18102 \
  --api-key local-secret \
  --rss-pid 21173 \
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
| 1 | 1 | 1 | stop | 21688 | 1 | 19675 | 19675 | 0.050825 | 1491304448 | 1473691648 | 1473691648 |
| 2 | 1 | 1 | stop | 7885 | 1 | 5875 | 5875 | 0.170185 | 1473691648 | 1496350720 | 1496350720 |
| 3 | 1 | 1 | stop | 8653 | 1 | 6642 | 6642 | 0.150549 | 1496350720 | 1476788224 | 1476788224 |
| 4 | 1 | 1 | stop | 17103 | 1 | 15068 | 15068 | 0.066365 | 1476788224 | 1471348736 | 1471348736 |

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

After stopping the server, `lsof -nP -iTCP:18102 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has integrated stop-summary evidence for the Qwen2.5 1.5B Q6_K
OpenAI-compatible streaming chat path. The run proves the Q6 model can satisfy
the same stop, reconnect/error, RSS, and terminal timing summary shape already
proven for Qwen2.5 0.5B Q4_K_M and Qwen2.5 1.5B Q8_0.

Remaining proof gaps:

- Repeat integrated stop-summary runs for SmolLM2 1.7B Q4_K_M.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Repeat integrated full-length error/disconnect summary runs for the larger
  required Tier 1 models.
