# OpenAI Real Tier 1 Stop Test Split

Date: 2026-06-29

## Context

`openai_real_tier1_http.rs` mixed general ignored Qwen2.5-0.5B Tier 1 HTTP
proofs with stop-sequence-specific checks. That made the broad real-model HTTP
target larger and duplicated stop-stream assertions that already exist in
`support::stop_sequences`.

## Change

- Added `openai_real_tier1_stop.rs` for the two ignored Qwen2.5-0.5B stop
  sequence regressions:
  - non-streaming legacy completion and chat stop trimming;
  - streaming legacy completion and chat stop trimming.
- Reduced `openai_real_tier1_http.rs` to the general real-model completion,
  chat, streaming, and concurrency proofs.
- Reused `support::stop_sequences::{assert_stop_completion_stream,
  assert_stop_chat_stream}` for streamed stop assertions.

## Validation

Baseline before the split:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --list
```

Result:

- `openai_real_tier1_http`: 8 listed tests.

After the split:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --list
cargo test -p ferrite-server --test openai_real_tier1_stop -- --list
```

Results:

- `openai_real_tier1_http`: 6 listed tests.
- `openai_real_tier1_stop`: 2 listed tests.

The `--list` checks compile the ignored real-model targets without running the
heavy model-dependent tests.
