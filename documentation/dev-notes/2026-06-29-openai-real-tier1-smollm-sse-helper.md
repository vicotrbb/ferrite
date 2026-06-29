# OpenAI Real Tier 1 SmolLM SSE Helper

Date: 2026-06-29

## Context

The SmolLM2-1.7B Q4_K_M real Tier 1 chat and legacy-completion streaming
targets still carried local copies of the SSE JSON parser after
`support::http::sse_json_events` became available.

## Change

- Updated `openai_real_tier1_smollm_1_7b_chat.rs` to use
  `support::http::sse_json_events`.
- Updated `openai_real_tier1_smollm_1_7b_streaming.rs` to use
  `support::http::sse_json_events`.
- Left all ignored real-model expected streamed token text unchanged.

## Validation

Baseline before the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_chat -- --list
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_streaming -- --list
```

Results:

- `openai_real_tier1_smollm_1_7b_chat`: 3 listed tests.
- `openai_real_tier1_smollm_1_7b_streaming`: 1 listed test.

After the change:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_chat -- --list
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_streaming -- --list
```

Results:

- `openai_real_tier1_smollm_1_7b_chat`: 3 listed tests.
- `openai_real_tier1_smollm_1_7b_streaming`: 1 listed test.

The `--list` checks compile the ignored real-model targets without running the
heavy model-dependent tests.
