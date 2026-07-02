# Streaming Cached-Token Observability

Date: 2026-07-02

## Slice

This slice makes the throughput client and long-chat gate report
`prompt_tokens_details.cached_tokens` from OpenAI-compatible streaming usage
chunks.

It prepares the proof gate for ADR 0009 without implementing K/V reuse.

## Implementation

- Added `StreamingUsageSummary::cached_prompt_tokens`.
- Parsed nested `prompt_tokens_details.cached_tokens` from SSE usage chunks.
- Kept older usage chunks compatible by defaulting missing cached-token details
  to `0`.
- Added `streaming_usage_cached_prompt_tokens` to throughput output.
- Added `long_chat_result_usage_cached_prompt_tokens` to long-chat scenario
  output.

## Red Test

The initial focused test failed before implementation with:

```text
error[E0599]: no method named `with_cached_prompt_tokens` found for struct `streaming_usage::StreamingUsageSummary`
error[E0599]: no method named `cached_prompt_tokens` found for struct `streaming_usage::StreamingUsageSummary`
```

## Validation

Focused checks:

```sh
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server extracts_streaming_usage_from_sse_body -- --nocapture
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server formats_streaming_usage_summary -- --nocapture
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server --test long_chat_gate formats_long_chat_scenario_result -- --nocapture
```

Package checks:

```sh
CARGO_TARGET_DIR=target/codex-cache-options cargo test -p ferrite-server --lib
cargo fmt --all -- --check
git diff --check
```

Results:

- Streaming usage extraction test: passed.
- Streaming usage formatting test: passed.
- Long-chat scenario formatting test: passed.
- `cargo test -p ferrite-server --lib`: 357 passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Limits

Ferrite still reports `cached_tokens = 0` on real request paths because no K/V
reuse exists yet. This slice only ensures future long-chat and throughput
proofs can observe cached-token accounting once real reuse is implemented.
