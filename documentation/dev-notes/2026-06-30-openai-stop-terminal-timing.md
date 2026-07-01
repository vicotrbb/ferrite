# OpenAI Stop Terminal Timing

## Slice

The integrated long-chat stop proof showed four valid `finish_reason=stop`
responses, but the run summary reported `long_chat_summary_all_timing_present=false`.
The throughput client only counted SSE chunks with visible generated text as
timing events. That excluded terminal stop chunks when the configured stop
sequence filtered the generated token out of visible stream content.

This slice updates the streaming timing extractor to treat an SSE JSON chunk
with a non-null `finish_reason` as a timing signal. Role-only chunks and
`[DONE]` still do not count.

## Validation

```sh
cargo test -p ferrite-server --lib derives_streaming_timing_from_terminal_stop_event_without_visible_content -- --nocapture
cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Results:

- The new test first failed with `expected streaming timing summary`.
- After the extractor change, the same test passed.
- Full `throughput_client::tests`: 43 passed.
- Full `long_chat_gate` integration target: 21 passed.
- `ferrite-server` clippy across all targets: passed.

## Remaining Scope

This preserves the existing `streaming_token_events` field name, so stop
streams can now report a timing event for the terminal stop chunk even when no
visible content was emitted. A broader protocol cleanup could later distinguish
visible token events from terminal timing events in the output schema.
