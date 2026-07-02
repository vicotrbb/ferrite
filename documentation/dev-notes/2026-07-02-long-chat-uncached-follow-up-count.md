# Long-Chat Uncached Follow-Up Count

Date: 2026-07-02

## Goal

Make cache-required long-chat failures quantify how many generated follow-up
turns did not report cached prompt tokens.

## Context

The previous slice added generated follow-up and cached generated follow-up
counts:

```text
long_chat_summary_generated_follow_up_turns=...
long_chat_summary_cached_generated_follow_up_turns=...
```

That made cache coverage auditable, but failure notes still had to subtract the
two values manually to describe how many generated follow-up turns missed cache
evidence.

## Changes

Added:

```text
long_chat_summary_uncached_generated_follow_up_turns=...
```

For a cache-required four-turn run where none of the three generated follow-up
turns report cached prompt tokens, the summary now records:

```text
long_chat_summary_generated_follow_up_turns=3
long_chat_summary_cached_generated_follow_up_turns=0
long_chat_summary_uncached_generated_follow_up_turns=3
long_chat_summary_all_generated_follow_up_turns_cached=false
long_chat_summary_run_complete=false
```

## Red Test

The focused test first failed because the uncached count field was absent:

```text
assertion failed: summary.contains("long_chat_summary_uncached_generated_follow_up_turns=3")
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-uncached-count cargo test -p ferrite-server --test long_chat_gate required_cached_follow_ups_make_summary_incomplete_without_cache_hits -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-uncached-count cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 31 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Limits

This is summary metadata only. It did not run a real model, did not run
`llama-benchy`, and did not prove prefix-cache latency, memory, or throughput
improvements.
