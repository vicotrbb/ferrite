# Dev Note: Long-Chat Finish Source Gate

Date: 2026-07-03

## Purpose

Close the proof-tooling gap exposed by the natural-EOS probe. Before this
slice, Ferrite's OpenAI-compatible surface reported `finish_reason=stop`, but
the long-chat gate could not distinguish tokenizer EOS from explicit stop
sequences. That made EOS closure ambiguous.

## Change

- Added `GenerationFinishSource` to the runtime:
  - `length`
  - `eos`
  - `generation_control`
  - `stop_sequence`
- Runtime generation now marks tokenizer EOS as `eos`.
- OpenAI stop-sequence filtering marks explicit stop-string termination as
  `stop_sequence`.
- Streaming usage now includes:

```text
completion_tokens_details.ferrite_finish_source
```

- The throughput client parses that field as `StreamingUsageSummary`.
- The long-chat gate prints:

```text
long_chat_result_finish_source=...
```

- The long-chat gate accepts:

```text
--require-finish-sources length,eos,stop_sequence,generation_control
```

and reports:

```text
long_chat_summary_required_finish_sources=...
long_chat_summary_required_finish_sources_present=true|false
```

`long_chat_summary_run_complete=true` now requires every configured finish
source to appear in the completed scenario results.

## Validation

Red phase:

```sh
cargo test -p ferrite-server --test long_chat_gate required_finish_sources_participate_in_long_chat_summary -- --nocapture
```

Failed before implementation because `LongChatGateConfig::required_finish_sources`
and `StreamingUsageSummary::with_finish_source` did not exist.

Green phase:

```sh
cargo test -p ferrite-server runtime::tests::generate_marks_eos_finish_source -- --nocapture
cargo test -p ferrite-server throughput_client::tests::extracts_streaming_usage_finish_source_from_sse_body -- --nocapture
cargo test -p ferrite-server --test long_chat_gate required_finish_sources_participate_in_long_chat_summary -- --nocapture
cargo test -p ferrite-server --lib
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Observed:

- runtime focused test passed;
- throughput finish-source parser test passed;
- long-chat finish-source requirement test passed;
- `cargo test -p ferrite-server --lib` passed, `398` tests;
- `cargo test -p ferrite-server --test long_chat_gate -- --nocapture` passed,
  `71` tests.

## Boundary

This implements proof observability and gate enforcement. It does not by itself
prove a real Qwen or SmolLM EOS run. The next proof run should use
`--require-finish-sources eos` once a deterministic tokenizer-EOS fixture or
real-model prompt shape is available.
