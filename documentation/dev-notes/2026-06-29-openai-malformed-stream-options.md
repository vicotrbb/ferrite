# OpenAI Malformed Stream Options

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat and legacy completion streaming endpoints now
return explicit `stream_options.include_usage` validation errors when
`include_usage` is present with a malformed type.

OpenAI clients may send `stream_options` when streaming responses and
`include_usage` is the option Ferrite currently supports. A malformed nested
value should not fail as a generic request-body deserialization error because
that hides the field users need to fix.

## Red

Added route tests for:

- `POST /v1/chat/completions` with
  `"stream": true, "stream_options": {"include_usage": "yes"}`
- `POST /v1/completions` with
  `"stream": true, "stream_options": {"include_usage": "yes"}`

Initial focused run:

```text
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
```

Both new tests failed with the existing generic error:

```text
Failed to deserialize the JSON body into the target type
```

## Implementation

- Replaced derived `StreamOptions` deserialization with explicit JSON-value
  parsing in `crates/ferrite-server/src/openai/schema/stream_options.rs`.
- Preserved supported `include_usage: true` behavior.
- Recorded malformed `include_usage`, malformed `include_obfuscation`, and
  non-object `stream_options` as validation state instead of aborting body
  deserialization.
- Added a request-field reporting helper so chat and legacy completions emit
  already-prefixed fields such as `stream_options.include_usage`.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server openai::stream_options_tests -- --nocapture
```

Result:

```text
8 passed; 0 failed; 0 ignored
```

## Limits

This slice does not add new streaming behavior or implement additional OpenAI
stream options. It only improves request validation so malformed documented
stream-option fields return OpenAI-shaped errors through the normal unsupported
field path.
