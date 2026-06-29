# OpenAI Real Tier 1 SSE Test Helper

Date: 2026-06-29

## Context

The Qwen2.5-1.5B Q8_0 and Q6_K prompt regression targets both carried the
same local `sse_json_events` parser for OpenAI-compatible SSE responses.
Keeping that parser duplicated increases drift risk as more ignored real-model
HTTP targets assert streamed chunks.

## Change

- Added `support::http::sse_json_events`.
- Updated `openai_real_tier1_qwen_1_5b_prompts.rs` and
  `openai_real_tier1_qwen_1_5b_q6_prompts.rs` to use the shared helper.
- Left the ignored real-model assertions and expected token/content values
  unchanged.

## Validation

Baseline before the helper extraction:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts -- --list
```

Results:

- `openai_real_tier1_qwen_1_5b_prompts`: 3 listed tests.
- `openai_real_tier1_qwen_1_5b_q6_prompts`: 3 listed tests.

After the helper extraction:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts -- --list
```

Results:

- `openai_real_tier1_qwen_1_5b_prompts`: 3 listed tests.
- `openai_real_tier1_qwen_1_5b_q6_prompts`: 3 listed tests.

The `--list` checks compile the ignored real-model targets without running the
heavy model-dependent tests.
