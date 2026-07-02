# Long-Chat Cache Follow-Up Counts

Date: 2026-07-02

## Goal

Make cache-keyed long-chat summaries expose count-based generated follow-up
cache evidence, not only boolean cache coverage.

## Context

The integrated long-chat summary already reported:

```text
long_chat_summary_any_cached_prompt_tokens=...
long_chat_summary_all_generated_follow_up_turns_cached=...
```

Those booleans help with pass/fail gating, but they do not show whether a run
had zero, partial, or all generated follow-up turns cached. That distinction is
important for future `--prompt-cache-key`, `--require-cached-follow-ups`, and
`llama-benchy` comparison notes.

## Changes

Added summary fields:

```text
long_chat_summary_generated_follow_up_turns=...
long_chat_summary_cached_generated_follow_up_turns=...
```

The existing `long_chat_summary_all_generated_follow_up_turns_cached` field now
uses the same counts internally, so it remains false when no generated follow-up
turns are present and true only when every generated follow-up turn has cached
prompt tokens.

## Red Test

The focused test first failed because the summary did not include the count
field:

```text
assertion failed: summary.contains("long_chat_summary_generated_follow_up_turns=3")
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-counts cargo test -p ferrite-server --test long_chat_gate formats_cache_observability_in_long_chat_run_summary -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-counts cargo test -p ferrite-server --test long_chat_gate -- --nocapture
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

This is proof-harness summary metadata only. It did not run a real model, did
not run `llama-benchy`, and did not prove cache reuse improves latency, RSS, or
throughput.
