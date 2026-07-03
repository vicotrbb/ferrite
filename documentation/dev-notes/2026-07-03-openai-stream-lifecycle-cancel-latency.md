# OpenAI Stream Lifecycle Cancel Latency

Date: 2026-07-03

## Goal

Make the OpenAI streaming lifecycle log more useful for cancellation theory
testing. The prior lifecycle line reported total request elapsed time and the
disconnect point, but it did not report whether prompt-cancellation polls
actually observed a closed stream or how long elapsed between first observed
disconnect and final lifecycle logging.

## Change

`StreamLifecycle` now records:

- `prompt_cancellation_closed_polls`: prompt-cancellation polls that observed a
  closed stream;
- `disconnect_observed_elapsed_ms`: elapsed request time when the closed stream
  was first observed;
- `disconnect_to_finish_ms`: elapsed time between first observed disconnect and
  final request lifecycle summary;
- `prompt_cancellation_token_index`: prompt token index active when cancellation
  observed the closed stream;
- `prompt_cancellation_layer_index`: transformer layer active when cancellation
  observed the closed stream, or `none` when cancellation happened before layer
  execution.

The emitted lifecycle line now includes these fields:

```text
openai_stream_lifecycle ... prompt_cancellation_closed_polls=N ... disconnect_observed_elapsed_ms=N|none disconnect_to_finish_ms=N|none prompt_cancellation_token_index=N|none prompt_cancellation_layer_index=N|none
```

Completed requests with no disconnect report `disconnect_to_finish_ms=none`.
They also report `disconnect_observed_elapsed_ms=none` and no prompt
cancellation location.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server stream_lifecycle -- --nocapture
error[E0609]: no field `prompt_cancellation_closed_polls` on type `StreamLifecycleSummary`
error[E0609]: no field `disconnect_to_finish_ms` on type `StreamLifecycleSummary`
```

Green test evidence:

```text
cargo test -p ferrite-server stream_lifecycle -- --nocapture
test openai::stream_lifecycle::tests::lifecycle_summary_records_prompt_generation_and_disconnect_state ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out
```

Second red test evidence:

```text
cargo test -p ferrite-server stream_lifecycle -- --nocapture
error[E0609]: no field `disconnect_observed_elapsed_ms` on type `StreamLifecycleSummary`
```

Second green test evidence:

```text
cargo test -p ferrite-server stream_lifecycle -- --nocapture
test openai::stream_lifecycle::tests::lifecycle_summary_records_prompt_generation_and_disconnect_state ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out
```

Third red test evidence:

```text
cargo test -p ferrite-server prompt_cancellation_poll_reports_prompt_location -- --nocapture
error[E0593]: closure is expected to take 0 arguments, but it takes 1 argument
```

Third green test evidence:

```text
cargo test -p ferrite-server prompt_cancellation_poll_reports_prompt_location -- --nocapture
test runtime::tests::prompt_cancellation_poll_reports_prompt_location ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 392 filtered out
```

Inference compatibility check:

```text
cargo test -p ferrite-inference --test scalar_prompt_cancellation -- --nocapture
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Formatting:

```text
cargo fmt
cargo fmt -- --check
git diff --check
```

## Limits

This is lifecycle observability only. It does not change cancellation policy,
prompt evaluation, streaming behavior, or OpenAI response shape. The next
real-model cancellation proof must rerun the prefill-disconnect scenario before
these fields can support a performance or latency claim.

## Real-Model Follow-Up

The observed-elapsed field was exercised against local Qwen2.5-0.5B Q4_K_M:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-prefill-cancel-observed-elapsed.md`
- `disconnect_observed_elapsed_ms=6495`
- `disconnect_to_finish_ms=0`
- `generated_chunks=0`
- `generated_token_ids=0`

The result shows the remaining local delay sits before the server observes the
closed stream. The next useful lifecycle fields are prompt token index and
transformer layer index at cancellation.

The prompt-token and layer fields have now been added to the lifecycle log. The
same cancellation scenario was rerun against local Qwen2.5-0.5B Q4_K_M:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-prefill-cancel-location.md`
- `disconnect_observed_elapsed_ms=8203`
- `disconnect_to_finish_ms=0`
- `prompt_cancellation_token_index=0`
- `prompt_cancellation_layer_index=none`

That proof places the remaining delay before the first prompt-token layer
evaluation, not inside one transformer layer.
