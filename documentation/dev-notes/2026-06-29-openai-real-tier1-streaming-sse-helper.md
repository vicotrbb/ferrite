# OpenAI Real Tier 1 Streaming SSE Helper

Date: 2026-06-29

## Context

The dedicated Qwen2.5-1.5B Q8_0 and Q6_K streaming prompt regression targets
still carried local copies of the SSE JSON parser after `support::http`
gained the shared `sse_json_events` helper.

## Change

- Updated `openai_real_tier1_qwen_1_5b_streaming_prompts.rs` to use
  `support::http::sse_json_events`.
- Updated `openai_real_tier1_qwen_1_5b_q6_streaming_prompts.rs` to use
  `support::http::sse_json_events`.
- Left all ignored real-model expected streamed token text unchanged.

## Validation

Baseline before the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_streaming_prompts -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_streaming_prompts -- --list
```

Results:

- `openai_real_tier1_qwen_1_5b_streaming_prompts`: 1 listed test.
- `openai_real_tier1_qwen_1_5b_q6_streaming_prompts`: 1 listed test.

After the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_streaming_prompts -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_streaming_prompts -- --list
```

Results:

- `openai_real_tier1_qwen_1_5b_streaming_prompts`: 1 listed test.
- `openai_real_tier1_qwen_1_5b_q6_streaming_prompts`: 1 listed test.

The `--list` checks compile the ignored real-model targets without running the
heavy model-dependent tests.
