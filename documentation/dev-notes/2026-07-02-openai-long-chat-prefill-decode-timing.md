# OpenAI long-chat prefill/decode timing split

## Context

The generated-context Qwen2.5-1.5B Q8_0 proofs show large follow-up
time-to-first-token growth as prompt tokens increase. Before implementing prefix
reuse or context windowing, the long-chat gate needs a measurement-only timing
split that can distinguish first-token delay from post-first-token streaming
pace.

## Slice

Added stream-observed timing fields to the long-chat scenario result formatter:

- `long_chat_result_stream_observed_prefill_elapsed_ms`
- `long_chat_result_first_token_timestamp_ms`
- `long_chat_result_stream_observed_decode_elapsed_ms`
- `long_chat_result_stream_observed_decode_tokens_per_second`

The fields are derived from existing SSE token event offsets. They are
client-observed measurements:

- prefill elapsed is currently the same duration as time to first streamed
  token;
- first-token timestamp is the same stream-relative offset;
- decode elapsed is total observed streaming elapsed after the first token;
- decode tokens per second counts token events after the first token.

This does not claim an internal engine prefill/decode split. It gives the proof
harness a stable output contract for comparing seed and generated-context turns.

## TDD

RED:

- `cargo test -p ferrite-server --test long_chat_gate formats_long_chat_scenario_result -- --exact`

Observed failure:

- `format_scenario_result` did not emit the new stream-observed prefill/decode
  fields.

GREEN:

- Added derived timing accessors to `StreamingTimingSummary`.
- Emitted the new long-chat result fields when streaming timing is present.
- Covered normal multi-token and one-token stream cases without divide-by-zero
  decode throughput.

## Validation

Executed:

- `cargo test -p ferrite-server --test long_chat_gate formats_long_chat_scenario_result -- --exact`
- `cargo test -p ferrite-server throughput_client::tests::summarizes_streaming_token_arrival_latencies -- --exact`
- `cargo test -p ferrite-server throughput_client::tests::waits_for_completed_sse_event_before_recording_streaming_timing -- --exact`
- `cargo test -p ferrite-server --test long_chat_gate`
- `cargo fmt --all -- --check`

Result:

- focused long-chat formatter test passed;
- focused throughput timing tests passed;
- full long-chat gate integration test passed: 24 passed, 0 failed;
- formatting check passed.

## Next Proof Step

Rerun a long-chat generated-context probe and record the new fields in the
benchmark note. The theory is only strengthened if generated-context turns show
that most of the regression sits before the first streamed token while
post-first-token decode pace remains comparatively stable.
