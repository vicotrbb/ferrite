# OpenAI Assistant Refusal Null

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts assistant transcript
messages with top-level `refusal: null` as a local no-op.

OpenAI documents assistant messages with optional top-level `refusal` metadata.
Ferrite already accepts refusal text when it is represented as a supported
assistant content part, but clients may also serialize the optional top-level
metadata as `null` in text-only transcripts. That null value carries no refusal
state and should not invalidate the local transcript.

Reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Red

The focused route test first sent:

```json
{"role":"assistant","content":"hello","refusal":null}
```

through `POST /v1/chat/completions`:

```sh
cargo test -p ferrite-server null_assistant_refusal_message_metadata -- --nocapture
```

It failed before implementation with:

```text
unsupported chat completion field(s): messages.refusal
```

## Green

Changes:

- `ChatMessage` now owns a message-level `refusal` field instead of treating it
  as an unknown extra field.
- Missing and `null` refusal metadata values remain neutral.
- Non-null top-level refusal metadata still reports `messages.refusal` as
  unsupported.

Verification:

```sh
cargo test -p ferrite-server null_assistant_refusal_message_metadata -- --nocapture
cargo test -p ferrite-server assistant_refusal_metadata_string -- --nocapture
```

## Boundary

This slice does not implement hosted refusal semantics, policy behavior, or a
second top-level refusal text path. Refusal text remains supported through the
existing assistant content-part parser; only the optional null top-level
metadata shape is accepted here.
