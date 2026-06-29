# OpenAI Non-Object Message Items

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat completion endpoint now preserves non-object
items inside the `messages` array long enough to return explicit
message-field validation errors.

## Red

Added a route test for:

```json
{
  "model": "fixture-model",
  "messages": [42]
}
```

Initial focused run:

```text
cargo test -p ferrite-server openai::unsupported_tests::chat_endpoint_rejects_non_object_message_items -- --nocapture
```

The new test failed with:

```text
messages must contain at least one item
```

That showed the chat message array deserializer discarded the malformed array
item and reported an empty input instead of a precise validation error.

## Implementation

- Added a narrow `ChatMessage::from_request_value` parser entrypoint for request
  JSON values.
- Kept valid JSON object messages on the normal `ChatMessage` deserialization
  path.
- Recorded non-object message items as malformed request-validation sentinels,
  which report `messages.role` and `messages.content` through the existing
  unsupported-field pipeline.
- Replaced the array-level `unwrap_or_default()` behavior with per-item
  preservation.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server openai::unsupported_tests::chat_endpoint_rejects_non_object_message_items -- --nocapture
cargo test -p ferrite-server chat_messages -- --nocapture
cargo test -p ferrite-server chat::tests::records_malformed_message_item_for_request_validation -- --nocapture
```

Results:

```text
chat_endpoint_rejects_non_object_message_items ... ok
openai::schema::chat_messages ... 4 passed; 0 failed
records_malformed_message_item_for_request_validation ... ok
```

## Limits

This slice does not add indexed OpenAI error parameters for individual message
array positions. It only prevents malformed array items from being collapsed
into a misleading empty-message error.
