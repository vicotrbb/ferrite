# OpenAI Text Response Format

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now accepts explicit
`response_format: {"type": "text"}` as a no-op. OpenAI documents text response
format as the default response format for chat completions, while JSON object
and JSON schema formats constrain the generated output and require additional
generation semantics.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/response_format.rs` to keep
  response-format compatibility detection separate from the chat request type.
- Updated chat unsupported-field detection to accept only missing
  `response_format` or `{"type": "text"}`.
- Kept JSON object and JSON schema response formats unsupported.
- Added a fixture-backed chat route test for explicit text response format.

## Red Test

```sh
cargo test -p ferrite-server text_response_format -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): response_format
```

## Validation

```sh
cargo test -p ferrite-server text_response_format -- --nocapture
cargo test -p ferrite-server openai::schema::response_format -- --nocapture
```

Both commands passed after implementation.

## Limits

This slice does not implement JSON mode, structured outputs, schema validation,
grammar-constrained decoding, or output repair. Those formats are not no-op
compatibility fields and remain unsupported until Ferrite has a tested design
for constrained local generation.
