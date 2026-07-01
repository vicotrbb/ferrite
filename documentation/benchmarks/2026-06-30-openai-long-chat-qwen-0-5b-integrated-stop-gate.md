# OpenAI Long-Chat Qwen 0.5B Integrated Stop Gate

## Scope

This run exercises the long-chat gate's explicit stop behavior through the
integrated summary path for Qwen2.5 0.5B Q4_K_M. It combines:

- explicit OpenAI `stop` sequence handling;
- expected `finish_reason=stop` assertion;
- four repeated turns;
- RSS sampling;
- terminal stop timing;
- request-level error recovery;
- client disconnect recovery;
- final `long_chat_summary_*` output.

This is a single-model stop proof. It does not prove EOS behavior and does not
close the full Tier 1 long-chat gate across all required models.

## Environment

- Date: 2026-06-30
- Commit: `54d3e77`
- Host: local macOS development machine
- Server port: `127.0.0.1:18100`
- Server PID for RSS sampling: `15403`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18100 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --error-probe \
  --disconnect-probe \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 1 \
  --turns 4 \
  --addr 127.0.0.1:18100 \
  --api-key local-secret \
  --rss-pid 15403 \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world' \
  --stop '1' \
  --expect-finish-reason stop
```

## Initial Finding

The first run after adding integrated summaries completed all four stop
scenarios with `finish_reason=stop`, but the summary reported:

```text
long_chat_summary_all_timing_present=false
long_chat_summary_run_complete=false
```

That exposed a real harness gap: the throughput client did not count terminal
`finish_reason=stop` chunks as timing events when the stop filter removed the
generated token from visible stream content. Commit `54d3e77` fixes that by
counting terminal finish chunks as timing signals.

## Rerun Results

Probe results:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
```

All four stop scenarios completed with `finish_reason=stop`. Usage counted one
generated token, and the stream emitted no visible content after the stop
boundary.

| Turn | Max tokens | Completed | Finish | Total ms | Stream events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 1 | stop | 6158 | 1 | 4136 | 4136 | 0.241727 | 401375232 | 401489920 | 401489920 |
| 2 | 1 | 1 | stop | 5178 | 1 | 3148 | 3148 | 0.317601 | 401489920 | 416890880 | 403013632 |
| 3 | 1 | 1 | stop | 4047 | 1 | 1989 | 1989 | 0.502708 | 403013632 | 426508288 | 280985600 |
| 4 | 1 | 1 | stop | 5042 | 1 | 2986 | 2986 | 0.334877 | 280985600 | 403570688 | 345341952 |

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

After stopping the server, `lsof -nP -iTCP:18100 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has a single-model integrated stop proof through the
OpenAI-compatible streaming chat path. The gate can require `finish_reason=stop`,
validate bounded usage accounting, capture terminal stop timing, include error
and disconnect probes, and emit a final machine-readable successful summary.

Remaining proof gaps:

- Repeat integrated stop-summary runs for the larger required Tier 1 models.
- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Repeat integrated full-length error/disconnect summary runs for the larger
  required Tier 1 models.
