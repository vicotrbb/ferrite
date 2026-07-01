# OpenAI Long-Chat SmolLM 1.7B Integrated Stop Gate

## Scope

This run exercises the long-chat gate's explicit stop behavior through the
integrated summary path for SmolLM2 1.7B Q4_K_M. It combines explicit OpenAI
`stop` sequence handling, expected `finish_reason=stop`, four repeated turns,
RSS sampling, terminal stop timing, request-level error recovery, client
disconnect recovery, and final `long_chat_summary_*` output.

This is a single-model stop proof. It does not prove EOS behavior and does not
close the full Tier 1 long-chat gate across 256, 512, and 1024-token streaming
responses.

## Environment

- Date: 2026-06-30
- Commit: `3228338`
- Host: local macOS development machine
- Server port: `127.0.0.1:18103`
- Server PID for RSS sampling: `25908`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18103 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --execute \
  --error-probe \
  --disconnect-probe \
  --disconnect-reconnect-timeout-ms 30000 \
  --models SmolLM2-1.7B-Instruct-Q4_K_M \
  --token-lengths 2 \
  --turns 4 \
  --addr 127.0.0.1:18103 \
  --api-key local-secret \
  --rss-pid 25908 \
  --prompt 'hello world' \
  --assistant-context 'short context' \
  --follow-up 'hello world' \
  --stop 'user' \
  --expect-finish-reason stop
```

The two-token stop variant is intentional. For this long-chat-shaped SmolLM
prompt, the first generated content token is a newline and the next generated
token is `user`. The earlier `stop: "1"` attempt correctly failed closed with
`finish_reason=length`, so it was not used as evidence.

## Results

Probe results:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
```

All four stop scenarios completed with `finish_reason=stop`. Usage counted two
generated tokens, and terminal stop timing was present for every turn.

| Turn | Max tokens | Completed | Finish | Total ms | Stream events | TTFT ms | Stream ms | Tok/s | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 2 | 1 | stop | 5960 | 2 | 3768 | 3950 | 0.506269 | 1078886400 | 1081802752 | 1081802752 |
| 2 | 2 | 1 | stop | 6457 | 2 | 4261 | 4445 | 0.449861 | 1081802752 | 1074954240 | 1074954240 |
| 3 | 2 | 1 | stop | 6036 | 2 | 3824 | 4025 | 0.496863 | 1074954240 | 1082703872 | 1082703872 |
| 4 | 2 | 1 | stop | 6085 | 2 | 3891 | 4075 | 0.490706 | 1082703872 | 1082032128 | 1082032128 |

Usage was stable for every turn:

- prompt tokens `20`;
- completion tokens `2`;
- total tokens `22`.

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

After stopping the server, `lsof -nP -iTCP:18103 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite now has integrated stop-summary evidence for the SmolLM2 1.7B Q4_K_M
OpenAI-compatible streaming chat path. The run proves SmolLM can satisfy the
same stop, reconnect/error, RSS, and terminal timing summary shape already
proven for the Qwen2.5 Tier 1 artifacts.

Remaining proof gaps:

- Add EOS-specific evidence once Ferrite exposes a distinct EOS terminal reason
  through the OpenAI-compatible stream.
- Run the full dedicated long-chat gate for every required Tier 1 model at
  256, 512, and 1024 streaming completion tokens.
- Repeat the full-length gate on x86_64 with bounded CPU and memory limits.
