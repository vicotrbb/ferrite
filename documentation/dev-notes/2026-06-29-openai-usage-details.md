# OpenAI Usage Details

Ferrite now serializes neutral token detail counters inside OpenAI-compatible
`usage` objects.

## Why

OpenAI's Chat Completions response examples include `prompt_tokens_details`
and `completion_tokens_details` alongside the aggregate `prompt_tokens`,
`completion_tokens`, and `total_tokens` counters. Ferrite does not currently
implement cached, audio, reasoning, or prediction-token accounting, so these
detail counters are reported as zero while keeping aggregate counts unchanged.

Reference:
https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Changes

- Added typed `prompt_tokens_details` and `completion_tokens_details` fields to
  the shared OpenAI `Usage` schema.
- Kept detail counters neutral and explicit:
  - cached tokens: `0`
  - audio tokens: `0`
  - reasoning tokens: `0`
  - accepted prediction tokens: `0`
  - rejected prediction tokens: `0`
- Added response-shape coverage for the chat endpoint.

## TDD Evidence

Red test:

```bash
cargo test -p ferrite-server chat_endpoint_returns_openai_message_shape -- --nocapture
```

Expected failure before implementation:

```text
Error: "expected prompt token details"
```

Focused green checks:

```bash
cargo test -p ferrite-server chat_endpoint_returns_openai_message_shape -- --nocapture
cargo test -p ferrite-server response_shape -- --nocapture
```
