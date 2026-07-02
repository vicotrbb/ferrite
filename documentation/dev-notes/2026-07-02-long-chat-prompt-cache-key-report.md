# Long-Chat Prompt Cache Key Report

Date: 2026-07-02

## Goal

Make cache-keyed long-chat runs self-describing in the machine-readable plan
output.

## Context

The long-chat gate can now pass `--prompt-cache-key` into
`ferrite-openai-throughput`, and each scenario result already reports
`long_chat_result_usage_cached_prompt_tokens`.

The missing audit field was the run-level configured cache namespace. Without
that, a benchmark note could show cached-token behavior without the plan output
recording which namespace was requested.

## Changes

- Added `long_chat_prompt_cache_key=...` to `format_plan` when
  `--prompt-cache-key` is configured.
- Left the default plan output unchanged when no prompt cache key is present.

## Red Test

The focused report test first failed because the plan omitted the configured
cache key:

```text
left: "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_planned_scenarios=4"
right: "long_chat_models=fixture-model\nlong_chat_token_lengths=256\nlong_chat_turns=4\nlong_chat_prompt_cache_key=long-chat:prefix\nlong_chat_planned_scenarios=4"
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-report cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_prompt_cache_key -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-report cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 26 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Results

Long-chat proof output now records the configured prompt-cache namespace before
scenario result lines. That makes future `--experimental-prefix-cache` proof
notes easier to audit alongside cached prompt-token usage.

## Limits

This slice did not run a real model and did not prove cache speedup. It only
improves proof-run metadata.
