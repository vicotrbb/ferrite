# Runtime Exact Prefix Cache Reuse

Date: 2026-07-02

## Slice

This slice adds opt-in exact-prefix cache reuse in `ferrite-server` runtime
generation.

It is not enabled by default and is not yet wired to OpenAI request metadata as
automatic behavior. OpenAI `prompt_cache_key` continues to provide namespace
metadata only unless runtime code explicitly enables prefix caching.

## Implementation

- Added `GenerationCacheOptions::with_prefix_cache_enabled(bool)`.
- Added a private runtime `RuntimePrefixCache`.
- Bounded the runtime cache by:
  - 8 entries;
  - 64 MiB estimated K/V bytes.
- Stored cache metadata through `PrefixCacheMetadataStore`.
- Stored runtime values as:
  - `ScalarLlamaSessionSnapshot`;
  - the `NextToken` produced by evaluating the prompt.
- On an enabled exact-key hit:
  - restore the prompt snapshot into a fresh scalar session;
  - reuse the stored prompt `NextToken` as the first generated token;
  - report the restored snapshot token count as
    `GeneratedText::cached_prompt_tokens()`.
- On a miss:
  - evaluate the full prompt normally;
  - snapshot the scalar session after prompt evaluation;
  - store the snapshot plus prompt `NextToken`;
  - report `cached_prompt_tokens = 0`.

The cache key remains the token-exact runtime key introduced in the previous
slice, including model, tokenizer, template, execution, request-shape, token
prefix, and optional namespace fingerprints.

## Red Test

The focused test was written before implementation:

```sh
CARGO_TARGET_DIR=target/codex-runtime-exact-prefix-cache cargo test -p ferrite-server runtime::tests::exact_prefix_cache_reuses_prompt_snapshot_when_enabled -- --nocapture
```

It failed for the expected missing API:

```text
error[E0599]: no method named `with_prefix_cache_enabled` found for struct `cache_options::GenerationCacheOptions`
```

## Validation

Focused check after implementation:

```sh
CARGO_TARGET_DIR=target/codex-runtime-exact-prefix-cache cargo test -p ferrite-server runtime::tests::exact_prefix_cache_reuses_prompt_snapshot_when_enabled -- --nocapture
```

Result: 1 passed.

Server library check:

```sh
CARGO_TARGET_DIR=target/codex-runtime-exact-prefix-cache cargo test -p ferrite-server --lib
```

Result: 359 passed.

Scalar session cache regression check:

```sh
CARGO_TARGET_DIR=target/codex-runtime-exact-prefix-cache cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
```

Result: 3 passed.

Formatting and whitespace checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo fmt --all -- --check`: passed after applying `cargo fmt --all`.
- `git diff --check`: passed.

## Limits

This is a fixture-level correctness proof, not a real-model latency or memory
claim.

The runtime cache currently stores cloned scalar snapshots. That is acceptable
for exact-prefix correctness proof, but it is not the final memory-efficient
K/V ownership model for larger models.

The cache is exact-key only. It does not implement longest-prefix matching,
partial reuse, sliding windows, cache promotion, cache invalidation beyond the
runtime model instance, or public server configuration.

Streaming and non-streaming runtime calls use the same generation path, but no
real OpenAI long-chat gate has been rerun with this option enabled.
