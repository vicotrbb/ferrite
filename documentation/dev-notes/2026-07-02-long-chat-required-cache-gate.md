# Long-Chat Required Cache Gate

Date: 2026-07-02

## Goal

Add an opt-in long-chat proof mode that requires generated follow-up turns to
observe cached prompt tokens.

This makes prefix-cache proof runs fail loudly when cache-keyed requests do not
actually hit the cache, while leaving normal long-chat correctness runs
unchanged.

## Context

The long-chat gate can already:

- pass `--prompt-cache-key` into chat-completion throughput requests;
- print the configured cache key in the plan;
- summarize whether cached prompt tokens were observed.

The remaining gap was that a cache-specific proof run could still report
`long_chat_summary_run_complete=true` even if every follow-up turn reported
`cached_tokens = 0`.

## Changes

- Added `--require-cached-follow-ups` to `ferrite-openai-long-chat-gate`.
- Added `LongChatGateConfig::require_cached_follow_ups()`.
- Added `long_chat_summary_cached_follow_ups_required=...`.
- Updated `long_chat_summary_run_complete` so missing generated follow-up cache
  hits make the run incomplete only when the new flag is set.

## Red Tests

The first focused runs failed because the config flag/accessor did not exist:

```text
error[E0599]: no method named `require_cached_follow_ups` found for struct `LongChatGateConfig`
```

After the config field was added, the full long-chat suite exposed the expected
summary-contract update:

```text
left:  ... long_chat_summary_cached_follow_ups_required=false ...
right: ... missing cached_follow_ups_required field ...
```

## Validation

Focused required-cache check:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-required cargo test -p ferrite-server --test long_chat_gate required_cached_follow_ups_make_summary_incomplete_without_cache_hits -- --nocapture
```

Result: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-required cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result: 28 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- formatting check: passed.
- whitespace check: passed.

## Results

Cache-keyed proof runs can now opt into a stricter completion condition:
generated follow-up turns must report cached prompt tokens, or
`long_chat_summary_run_complete=false`.

## Limits

This is a proof-harness gate only. It did not run a real model, did not run
`llama-benchy`, and did not prove cache speedup, response equivalence, or RSS
stability for a real prefix-cache run.
