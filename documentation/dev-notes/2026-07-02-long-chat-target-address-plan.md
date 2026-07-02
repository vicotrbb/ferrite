# Long-Chat Target Address Plan Metadata

Date: 2026-07-02

## Goal

Make long-chat proof logs show which OpenAI-compatible server address the gate
was configured to target.

## Context

The long-chat gate sends requests to `--addr`, defaulting to
`127.0.0.1:8080`. Before this slice, `format_plan` did not print the target
address, so archived proof output could show models, token lengths, and probes
without recording the target endpoint in the machine-readable plan block.

## Changes

Added a plan field:

```text
long_chat_addr=...
```

The plan prints the address for both default and custom configurations. It does
not print `--api-key`.

## Red Test

The focused test first failed because the custom address was omitted:

```text
left:  "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_planned_scenarios=4"
right: "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_addr=127.0.0.1:18080\nlong_chat_planned_scenarios=4"
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-addr-plan cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_server_address -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-addr-plan cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 35 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Limits

This is plan metadata only. It did not execute OpenAI-compatible requests, start
a server, run a real model, or run `llama-benchy`.
