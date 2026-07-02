# Long-Chat Probe Plan Metadata

Date: 2026-07-02

## Goal

Make RSS, error-probe, and disconnect-probe settings visible in the long-chat
gate's machine-readable plan output.

## Context

The Tier 1 long-chat proof gate requires RSS sampling, request-error behavior,
client disconnect behavior, and reconnect behavior. The long-chat gate already
accepts CLI options for those proof surfaces:

```text
--rss-pid ...
--error-probe
--disconnect-probe
--probe-max-tokens ...
--disconnect-reconnect-timeout-ms ...
```

Before this slice, `format_plan` did not include those settings, so benchmark
notes had to rely on the shell command instead of the plan block to determine
whether those proof probes were configured.

## Changes

Added optional plan fields:

```text
long_chat_rss_pid=...
long_chat_error_probe_required=true
long_chat_disconnect_probe_required=true
long_chat_probe_max_tokens=...
long_chat_disconnect_reconnect_timeout_ms=...
```

These fields are emitted only when the corresponding option is configured. The
disconnect reconnect timeout is emitted when `--disconnect-probe` is enabled,
because that timeout only affects the reconnect portion of the disconnect
probe.

## Red Test

The focused test first failed because the plan omitted the probe metadata:

```text
left:  "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_planned_scenarios=4"
right: "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_rss_pid=4242\nlong_chat_error_probe_required=true\nlong_chat_disconnect_probe_required=true\nlong_chat_probe_max_tokens=256\nlong_chat_disconnect_reconnect_timeout_ms=1500\nlong_chat_planned_scenarios=4"
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-probe-plan cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_probe_metadata -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-probe-plan cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 33 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Limits

This is plan metadata only. It did not run a real model, RSS sampling, error
probe, disconnect probe, reconnect probe, or `llama-benchy`.
