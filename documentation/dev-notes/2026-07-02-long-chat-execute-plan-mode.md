# Long-Chat Execute Plan Mode

Date: 2026-07-02

## Goal

Make the long-chat gate plan show whether the scenario matrix is configured to
execute or only print a plan and optional probes.

## Context

`ferrite-openai-long-chat-gate` always prints the plan and scenario matrix. It
only sends the repeated long-chat scenario requests when `--execute` is present.

Before this slice, archived output could show the planned scenarios but not
whether the run was configured to execute them.

## Changes

Added an optional plan field:

```text
long_chat_execute=true
```

The field is emitted only when `--execute` is configured. Dry plan output stays
unchanged.

## Red Test

The focused test first failed because the execute mode was absent:

```text
left:  "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_planned_scenarios=4"
right: "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_execute=true\nlong_chat_planned_scenarios=4"
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-execute-plan cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_execute_flag -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-execute-plan cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 34 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Limits

This is plan metadata only. It did not execute any OpenAI-compatible requests,
real model runs, RSS probes, reconnect probes, or `llama-benchy`.
