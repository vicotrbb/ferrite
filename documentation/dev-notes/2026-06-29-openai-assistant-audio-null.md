# OpenAI Assistant Audio Null

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts assistant transcript
messages with `audio: null` as a local no-op.

OpenAI documents assistant messages with optional `audio` metadata for previous
audio responses. Ferrite is a local text-generation server today and does not
implement audio replay, but `null` carries no audio state and should not make a
text-only transcript invalid.

Reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Red

The focused route test first sent:

```json
{"role":"assistant","content":"hello","audio":null}
```

through `POST /v1/chat/completions`:

```sh
cargo test -p ferrite-server null_assistant_audio_message_metadata -- --nocapture
```

It failed before implementation with:

```text
unsupported chat completion field(s): messages.audio
```

## Green

Changes:

- `ChatMessage` now owns a message-level `audio` field instead of treating it as
  an unknown extra field.
- Missing and `null` audio values deserialize to no local state and remain
  neutral.
- Non-null message audio values still report `messages.audio` as unsupported.

Verification:

```sh
cargo test -p ferrite-server null_assistant_audio_message_metadata -- --nocapture
cargo test -p ferrite-server assistant_audio_object -- --nocapture
```

## Boundary

This slice does not implement audio generation, previous-audio response replay,
or multimodal request handling. It only accepts the null optional metadata shape
that OpenAI-compatible clients may serialize in text-only transcripts.
