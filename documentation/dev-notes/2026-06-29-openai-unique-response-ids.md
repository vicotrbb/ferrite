# OpenAI Unique Response IDs

## Scope

Ferrite now generates unique OpenAI-compatible response IDs for chat
completions, legacy completions, and their SSE stream contexts even when
multiple responses are created in the same second.

The OpenAI API reference describes completion and chat completion `id` fields
as unique identifiers. Ferrite previously derived IDs only from the Unix
timestamp in seconds, so separate responses created within the same second could
share the same ID.

Reference pages:

- https://developers.openai.com/api/reference/resources/completions/methods/create/
- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/

## Red Evidence

```text
cargo test -p ferrite-server schema:: -- --nocapture

failures:
    openai::schema::chat::tests::chat_completion_response_ids_are_unique_within_the_same_second
    openai::schema::chat_stream::tests::chat_stream_context_ids_are_unique_between_streams_in_the_same_second
    openai::schema::completion::tests::completion_response_ids_are_unique_within_the_same_second
    openai::schema::completion_stream::tests::completion_stream_context_ids_are_unique_between_streams_in_the_same_second

left: "chatcmpl-ferrite-1782741010"
right: "chatcmpl-ferrite-1782741010"
left: "cmpl-ferrite-1782741010"
right: "cmpl-ferrite-1782741010"
```

## Change

- Added a small `schema::id` helper that appends a process-local atomic sequence
  to Ferrite response IDs.
- Updated non-streaming chat and legacy completion responses to use the helper.
- Updated chat and legacy completion stream contexts to use the helper once per
  stream, preserving one ID across all chunks in the stream.
- Added tests for same-second uniqueness and same-stream ID stability.

## Green Evidence

```text
cargo test -p ferrite-server schema:: -- --nocapture

test openai::schema::chat::tests::chat_completion_response_ids_are_unique_within_the_same_second ... ok
test openai::schema::chat_stream::tests::chat_stream_context_ids_are_unique_between_streams_in_the_same_second ... ok
test openai::schema::completion::tests::completion_response_ids_are_unique_within_the_same_second ... ok
test openai::schema::completion_stream::tests::completion_stream_context_ids_are_unique_between_streams_in_the_same_second ... ok

test result: ok. 89 passed; 0 failed; 0 ignored; 0 measured; 152 filtered out
```

## Boundary

This slice only strengthens response identity. It does not add stored
completion retrieval, multi-choice generation, or new sampling behavior.
