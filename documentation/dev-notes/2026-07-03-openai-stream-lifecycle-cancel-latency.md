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
- `disconnect_to_finish_ms`: elapsed time between first observed disconnect and
  final request lifecycle summary.

The emitted lifecycle line now includes both fields:

```text
openai_stream_lifecycle ... prompt_cancellation_closed_polls=N ... disconnect_to_finish_ms=N|none
```

Completed requests with no disconnect report `disconnect_to_finish_ms=none`.

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

Formatting:

```text
cargo fmt
```

## Limits

This is lifecycle observability only. It does not change cancellation policy,
prompt evaluation, streaming behavior, or OpenAI response shape. The next
real-model cancellation proof must rerun the prefill-disconnect scenario before
these fields can support a performance or latency claim.
