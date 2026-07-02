# Long-Chat Cache Summary

Date: 2026-07-02

## Goal

Expose cache-token observability in the integrated long-chat summary so
cache-keyed proof runs can be audited from the summary block, not only from
per-scenario result lines.

## Context

The long-chat gate already reports per-scenario cached prompt tokens through:

```text
long_chat_result_usage_cached_prompt_tokens=...
```

After adding `--prompt-cache-key` support and plan output, the missing summary
fields were:

- whether the run was configured with a prompt cache key;
- whether any cached prompt tokens were observed;
- whether generated follow-up turns observed cached prompt tokens.

These fields are informational. They do not change `long_chat_summary_run_complete`
because correctness, cache observability, and performance proof remain separate
claims.

## Changes

Added summary fields:

```text
long_chat_summary_prompt_cache_key_present=...
long_chat_summary_any_cached_prompt_tokens=...
long_chat_summary_all_generated_follow_up_turns_cached=...
```

## Red Test

The focused test first failed because the summary omitted cache-observability
fields:

```text
assertion failed: summary.contains("long_chat_summary_prompt_cache_key_present=true")
```

## Validation

Focused check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-summary cargo test -p ferrite-server --test long_chat_gate formats_cache_observability_in_long_chat_run_summary -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-summary cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 27 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Results

Future `--prompt-cache-key` long-chat proof runs now expose whether cached
prompt tokens appeared in any scenario and whether all generated follow-up
turns observed cached prompt tokens.

## Limits

This is summary metadata only. It did not run a real model, did not run
`llama-benchy`, and did not prove lower first-token latency or bounded RSS for
prefix-cache reuse.
