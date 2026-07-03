# OpenAI Streaming EOS Finish Source Fixture

Date: 2026-07-03

## Purpose

The long-chat gate can now require machine-readable finish sources, but that
needs deterministic OpenAI streaming coverage before using the field as proof
for real-model EOS behavior.

This slice adds fixture-backed coverage for tokenizer EOS on
`POST /v1/chat/completions` with `stream: true` and
`stream_options.include_usage: true`.

## Change

- Added `scalar_llama_chat_f32_gguf_fixture_with_eos_token_id` in the existing
  chat fixture module.
- Added a shared server-test helper for writing that chat EOS fixture.
- Added a route-level SSE test that forces token id `2` to be EOS, verifies no
  visible assistant content is emitted for that EOS token, verifies terminal
  `finish_reason: "stop"`, and verifies the usage chunk reports
  `completion_tokens_details.ferrite_finish_source = "eos"`.

## TDD Note

The first test attempt used the scalar EOS fixture on the chat prompt path and
failed before prompt evaluation with:

```text
failed to tokenize prompt: no atomic token matches input at byte offset 0
```

The fix was to add the EOS variant to the chat fixture module rather than
reuse the scalar fixture for a prompt shape it does not support.

## Validation

```text
cargo test -p ferrite-server openai::stop_sequences_tests::chat_stream_endpoint_reports_eos_finish_source_in_usage -- --nocapture
```

Result: passed.

## Boundary

This is fixture-level OpenAI streaming evidence. It proves the EOS finish-source
field can propagate through the chat SSE usage path, but it does not by itself
close real-model Qwen or SmolLM2 EOS behavior in the Tier 1 long-chat matrix.
