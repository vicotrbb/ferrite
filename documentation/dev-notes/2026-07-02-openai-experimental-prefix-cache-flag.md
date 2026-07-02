# OpenAI Experimental Prefix Cache Flag

Date: 2026-07-02

## Slice

This slice makes exact-prefix cache reuse reachable from the OpenAI-compatible
chat route behind an explicit server experiment flag.

The flag is intentionally opt-in:

```sh
ferrite-server --experimental-prefix-cache
```

Without this flag, `prompt_cache_key` remains namespace metadata only and does
not enable reuse.

## Implementation

- Added `ServerConfig::experimental_prefix_cache_enabled()`.
- Added `--experimental-prefix-cache` parsing and usage text.
- Added `ServerState::with_prefix_cache_enabled(bool)`.
- Carried the parsed config flag from `main` into `ServerState`.
- Added a chat-route helper that combines:
  - request `prompt_cache_key` namespace from `ChatCompletionRequest`;
  - server-level prefix-cache experiment enablement.
- Left legacy completions unchanged.

## Red Tests

The config/state tests first failed for missing APIs:

```text
error[E0599]: no method named `experimental_prefix_cache_enabled` found for struct `config::ServerConfig`
error[E0599]: no method named `prefix_cache_enabled` found for struct `state::ServerState`
error[E0599]: no method named `with_prefix_cache_enabled` found for struct `state::ServerState`
```

The OpenAI route test then failed for the expected behavior gap:

```text
assertion `left == right` failed
  left: Number(0)
 right: 4
```

That failure showed the second repeated chat request still reported
`prompt_tokens_details.cached_tokens = 0` before the route passed the server
experiment flag into runtime cache options.

## Validation

Focused route check:

```sh
CARGO_TARGET_DIR=target/codex-openai-prefix-cache-route cargo test -p ferrite-server openai::routes_tests::chat_endpoint_reports_cached_tokens_when_experimental_prefix_cache_is_enabled -- --nocapture
```

Result: 1 passed.

Focused config and state checks:

```sh
CARGO_TARGET_DIR=target/codex-openai-prefix-cache-route cargo test -p ferrite-server config::tests::parses_experimental_prefix_cache_flag -- --nocapture
CARGO_TARGET_DIR=target/codex-openai-prefix-cache-route cargo test -p ferrite-server state::tests::prefix_cache_is_explicitly_enabled -- --nocapture
```

Results:

- config flag check: 1 passed.
- state flag check: 1 passed.

Server library check:

```sh
CARGO_TARGET_DIR=target/codex-openai-prefix-cache-route cargo test -p ferrite-server --lib
```

Result: 362 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo fmt --all -- --check`: passed after applying `cargo fmt --all`.
- `git diff --check`: passed.

## Limits

This is fixture-level OpenAI route proof only.

No real model benchmark or long-chat gate was run with
`--experimental-prefix-cache`. The next proof milestone must rerun the dedicated
long-chat matrix with 256, 512, and 1024-token streaming responses, repeated
multi-turn conversations, RSS before/after/idle sampling, latency per token,
stop/EOS behavior, and reconnect/error behavior.

The cache is still exact-prefix only. It does not implement longest-prefix
reuse, partial-prefix reuse, memory pressure policy beyond the current bounded
metadata/value store, or a production default.
