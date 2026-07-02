# Long-Chat Required Cache Config Validation

Date: 2026-07-02

## Goal

Prevent invalid cache-proof configuration where generated follow-up cache hits
are required but no prompt cache key is configured.

## Context

`--require-cached-follow-ups` makes `long_chat_summary_run_complete=false`
unless generated follow-up turns report cached prompt tokens. Without
`--prompt-cache-key`, that stricter proof mode is not meaningful because there
is no explicit cache namespace for the OpenAI-compatible chat requests.

## Changes

- Added post-parse validation:

```text
--require-cached-follow-ups requires --prompt-cache-key
```

- Updated the positive custom config parse test to include
  `--prompt-cache-key long-chat:prefix` when using
  `--require-cached-follow-ups`.

## Red Test

The new focused config test first failed because parsing accepted the invalid
flag combination:

```text
expected error, got config: LongChatGateConfig { ... require_cached_follow_ups: true, ... prompt_cache_key: None ... }
```

## Validation

Focused checks:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-config cargo test -p ferrite-server --test long_chat_gate rejects_required_cached_follow_ups_without_prompt_cache_key -- --nocapture
CARGO_TARGET_DIR=target/codex-long-chat-cache-config cargo test -p ferrite-server --test long_chat_gate parses_custom_long_chat_token_lengths_turns_and_models -- --nocapture
```

Results:

- required-cache rejection test: 1 passed.
- custom config parse test: 1 passed.

Related long-chat suite:

```sh
CARGO_TARGET_DIR=target/codex-long-chat-cache-config cargo test -p ferrite-server --test long_chat_gate -- --nocapture
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

## Results

The cache-specific long-chat gate now rejects a configuration that could never
prove cache-keyed follow-up reuse.

## Limits

This is config validation only. It did not run a real model and did not prove
cache speedup, response equivalence, or memory behavior.
