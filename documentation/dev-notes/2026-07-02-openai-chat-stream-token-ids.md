# OpenAI Chat Stream Token IDs

## Goal

Return generated token IDs in OpenAI-compatible chat streaming content chunks so
external benchmark clients can avoid fallback tokenization.

## Context

After Ferrite accepted the `return_token_ids` request extension,
`llama-benchy 0.3.8` completed a minimal benchmark but printed:

```text
No token_ids in response, using local tokenization
```

Ferrite already had generated token IDs internally. The missing piece was
preserving the token IDs that correspond to each emitted UTF-8-safe streaming
text piece and exposing them on chat stream choices.

## Change

- Added a runtime token event callback that reports emitted text with the token
  IDs that produced that text.
- Stored generated token ID chunks alongside `GeneratedText::token_texts()`.
- Added `token_ids` to chat stream content choices.
- Emitted `token_ids` for no-stop chat streams.
- Omitted `token_ids` when string stop-sequence filtering rewrites visible
  chunks, because the filtered text may no longer map exactly to emitted model
  token IDs.

## Red Test

The first focused schema test failed before implementation because
`ChatCompletionStreamContext::token_with_ids` did not exist. Runtime tests also
failed before the `GeneratedText::token_id_chunks()` plumbing existed.

## Validation

```text
CARGO_TARGET_DIR=target/codex-chat-token-ids cargo test -p ferrite-server chat_stream_content_chunk_can_include_token_ids -- --nocapture
```

Result: 1 passed, 0 failed.

```text
CARGO_TARGET_DIR=target/codex-chat-token-ids cargo test -p ferrite-server generate_with_token_callback_reports_each_token_piece -- --nocapture
```

Result: 1 passed, 0 failed.

```text
CARGO_TARGET_DIR=target/codex-chat-token-ids cargo test -p ferrite-server chat_endpoint_streams_openai_sse_chunks -- --nocapture
```

Result: 1 passed, 0 failed.

```text
CARGO_TARGET_DIR=target/codex-chat-token-ids cargo test -p ferrite-server openai::route_streaming_tests -- --nocapture
CARGO_TARGET_DIR=target/codex-chat-token-ids cargo test -p ferrite-server openai::stop_sequences_tests -- --nocapture
CARGO_TARGET_DIR=target/codex-chat-token-ids cargo test -p ferrite-server runtime::tests -- --nocapture
```

Results: 4 passed, 10 passed, and 6 passed.

```text
cargo fmt --all -- --check
git diff --check
```

Result: both exited cleanly.

Live external smoke:

```text
uvx llama-benchy ... --pp 16 --tg 8 --runs 1 --concurrency 1 --latency-mode none ...
```

Result: command exited `0`, wrote
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.json`,
and did not print the previous local-tokenization fallback line.

## Limits

This is not the full `llama-benchy` protocol and not the long-chat proof gate.
Stop-filtered chat streams intentionally omit token IDs until Ferrite can
represent filtered-token provenance precisely.
