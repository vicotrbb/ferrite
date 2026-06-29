# OpenAI Text Modalities

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now accepts explicit
`modalities: ["text"]` as a no-op. OpenAI documents `["text"]` as the default
output modality for chat completions, while audio output is a separate behavior
that Ferrite does not implement.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/modalities.rs` to keep
  modality compatibility detection separate from the chat request type.
- Added a typed `modalities` field to `ChatCompletionRequest` so text-only
  modality requests are not treated as unknown extra fields.
- Updated chat unsupported-field detection to accept only missing modalities or
  exactly `["text"]`.
- Added a fixture-backed chat route test for explicit text-only modalities.

## Red Test

```sh
cargo test -p ferrite-server text_only_modalities -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): modalities
```

## Validation

```sh
cargo test -p ferrite-server text_only_modalities -- --nocapture
cargo test -p ferrite-server openai::schema::modalities -- --nocapture
```

Both commands passed after implementation.

## Limits

This slice does not implement audio output, multimodal response bodies, audio
configuration, or text-plus-audio response generation. `["audio"]`,
`["text", "audio"]`, malformed modality values, and empty modality arrays remain
unsupported.
