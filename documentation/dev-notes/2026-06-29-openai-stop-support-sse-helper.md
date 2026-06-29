# OpenAI Stop Support SSE Helper

Date: 2026-06-29

## Context

`support::stop_sequences` still carried its own local SSE JSON parser after
`support::http::sse_json_events` became the shared integration-test helper for
OpenAI-compatible SSE responses.

## Change

- Updated `support::stop_sequences` to import
  `support::http::sse_json_events`.
- Removed the local stop-support SSE parser.
- Left the stop-sequence assertions unchanged.

## Validation

Baseline before the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_stop -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q8_stop -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_stop -- --list
```

Results:

- `openai_real_tier1_stop`: 2 listed tests.
- `openai_real_tier1_qwen_1_5b_q8_stop`: 1 listed test.
- `openai_real_tier1_qwen_1_5b_q6_stop`: 1 listed test.

After the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_stop -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q8_stop -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_stop -- --list
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_stop -- --list
```

Results:

- `openai_real_tier1_stop`: 2 listed tests.
- `openai_real_tier1_qwen_1_5b_q8_stop`: 1 listed test.
- `openai_real_tier1_qwen_1_5b_q6_stop`: 1 listed test.
- `openai_real_tier1_smollm_1_7b_stop`: 1 listed test.

The `--list` checks compile the ignored real-model targets without running the
heavy model-dependent tests.
