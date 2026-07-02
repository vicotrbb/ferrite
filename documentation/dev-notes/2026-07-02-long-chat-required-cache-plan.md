# Long-Chat Required Cache Plan Output

Date: 2026-07-02

## Goal

Make the stricter cache proof mode visible in the long-chat plan output.

## Context

`--require-cached-follow-ups` changes the meaning of
`long_chat_summary_run_complete`: generated follow-up turns must report cached
prompt tokens. The summary already reports
`long_chat_summary_cached_follow_ups_required`, but the run plan did not show
that the stricter mode had been requested.

Benchmark notes usually capture the plan block before scenario output, so the
plan needs to be self-describing.

## Changes

When `--require-cached-follow-ups` is configured, `format_plan` now prints:

```text
long_chat_require_cached_follow_ups=true
```

The field is omitted for normal long-chat runs, preserving existing concise
plan output.

## Red Test

The focused plan-format test first failed because the plan omitted the required
cache gate mode:

```text
left:  "... long_chat_prompt_cache_key=long-chat:prefix\nlong_chat_planned_scenarios=4"
right: "... long_chat_prompt_cache_key=long-chat:prefix\nlong_chat_require_cached_follow_ups=true\nlong_chat_planned_scenarios=4"
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-required-plan cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_required_cached_follow_ups -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-required-plan cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 29 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Results

Cache-specific long-chat proof runs now show both the cache namespace and the
strict required-cache mode in the plan block.

## Limits

This is proof metadata only. It did not run a real model and did not prove
cache speedup, response equivalence, or RSS stability.
