# OpenAI Real Tier 1 Qwen Q8 Stop Test Split

Date: 2026-06-29

## Context

`openai_real_tier1_qwen_1_5b_http.rs` mixed general ignored
Qwen2.5-1.5B Q8_0 HTTP proofs with a stop-sequence-specific regression. The
matching Q6_K stop proof already lives in a focused target, so the Q8_0 layout
was inconsistent and kept the general HTTP target larger than necessary.

## Change

- Added `openai_real_tier1_qwen_1_5b_q8_stop.rs` for the ignored Q8_0
  stop-sequence regression covering:
  - non-streaming legacy completion stop trimming;
  - streaming legacy completion stop trimming;
  - non-streaming chat stop trimming;
  - streaming chat stop trimming.
- Reduced `openai_real_tier1_qwen_1_5b_http.rs` to general Q8_0 completion,
  chat, streaming, and bounded-wait proofs.
- Kept using `support::stop_sequences` for the shared stop assertions.

## Validation

Baseline before the split:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http -- --list
```

Result:

- `openai_real_tier1_qwen_1_5b_http`: 6 listed tests.

After the split:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http -- --list
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q8_stop -- --list
```

Results:

- `openai_real_tier1_qwen_1_5b_http`: 5 listed tests.
- `openai_real_tier1_qwen_1_5b_q8_stop`: 1 listed test.

The `--list` checks compile the ignored real-model targets without running the
heavy model-dependent tests.
