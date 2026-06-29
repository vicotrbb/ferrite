# OpenAI Chat Metadata

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts valid `metadata` objects.
The official Chat Completions API documents metadata as up to 16 structured
key-value pairs attached to the request object. Ferrite treats this as local
request metadata and does not pass it into the inference core.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/metadata.rs` to keep metadata
  validation focused and out of the chat request struct.
- Updated chat unsupported-field detection to accept only valid metadata
  objects.
- Added a fixture-backed route test for a valid metadata object.
- Added a route-level rejection regression for malformed metadata.

Accepted metadata must be:

- a JSON object;
- no more than 16 key-value pairs;
- string keys with at most 64 characters;
- string values with at most 512 characters.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_accepts_metadata_object -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): metadata
```

## Validation

```sh
cargo test -p ferrite-server chat_endpoint_accepts_metadata_object -- --nocapture
cargo test -p ferrite-server openai::schema::metadata -- --nocapture
```

Both commands passed after implementation.

## Limits

This slice does not persist metadata, expose metadata in responses, implement
dashboard querying, or use metadata in scheduling, safety, sampling, or
generation behavior. Metadata remains an HTTP protocol concern.
