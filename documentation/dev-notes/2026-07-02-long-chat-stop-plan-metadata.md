# Long-Chat Stop Plan Metadata

Date: 2026-07-02

## Goal

Make stop/EOS-focused long-chat runs self-describing in the machine-readable
plan output.

## Context

The long-chat gate already accepts:

```text
--stop ...
--expect-finish-reason ...
```

Those options are required for stop/EOS proof runs, but the plan output did not
show when a stop condition or expected terminal finish reason was configured.
That made benchmark notes depend on the shell command instead of the
machine-readable plan block.

## Changes

Added optional plan fields:

```text
long_chat_stop_configured=true
long_chat_expected_finish_reason=stop
```

The plan records only whether stop is configured, not the raw stop text. This
keeps generated plan lines compact and avoids embedding arbitrary stop strings
in the summary block.

## Red Test

The focused test first failed because the plan omitted both fields:

```text
left:  "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_planned_scenarios=4"
right: "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_stop_configured=true\nlong_chat_expected_finish_reason=stop\nlong_chat_planned_scenarios=4"
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-stop-plan cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_stop_expectation -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-stop-plan cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 32 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Limits

This is plan metadata only. It did not run a real stop/EOS model proof and did
not prove model-specific EOS behavior.
