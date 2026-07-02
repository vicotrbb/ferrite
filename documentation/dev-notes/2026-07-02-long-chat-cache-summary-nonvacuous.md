# Long-Chat Cache Summary Non-Vacuous Follow-Ups

Date: 2026-07-02

## Goal

Prevent incomplete cache-keyed long-chat runs from reporting generated follow-up
cache coverage as true when no generated follow-up result rows are present.

## Context

The long-chat summary reports:

```text
long_chat_summary_all_generated_follow_up_turns_cached=...
```

The previous implementation used `Iterator::all` over generated follow-up
results. For an incomplete run that only had a seed turn, that iterator was
empty, so the field could become true even though no generated follow-up cache
evidence existed.

That was misleading for cache-specific proof runs.

## Changes

- Added a focused summary regression test for a cache-keyed run with only the
  seed result present.
- Changed `all_generated_follow_up_turns_cached` so it requires at least one
  generated follow-up result before it can be true.
- Extracted the generated-follow-up predicate into a helper used by both the
  presence check and the cache-hit check.

## Red Test

The new focused test first failed because the summary still reported generated
follow-up cache coverage as true:

```text
assertion failed: summary.contains("long_chat_summary_all_generated_follow_up_turns_cached=false")
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-nonvacuous cargo test -p ferrite-server --test long_chat_gate cache_summary_does_not_treat_missing_generated_follow_ups_as_cached -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-nonvacuous cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 30 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Results

Cache-keyed long-chat summaries no longer treat missing generated follow-up
rows as successful generated-follow-up cache coverage.

## Limits

This is summary correctness only. It did not run a real model and did not prove
prefix-cache latency, memory, or response-shape behavior.
