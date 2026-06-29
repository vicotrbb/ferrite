# OpenAI Empty Stop Array Compatibility

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat and legacy completion endpoints now accept
`stop: []` as a semantics-neutral request option.

The OpenAI chat and legacy completion APIs document `stop` as an optional string
or array of strings. A non-empty stop sequence changes generation behavior and
remains unsupported until Ferrite implements stop-sequence truncation. An empty
array does not request any stop sequence, so this slice treats it like the field
being absent.

Source references:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/stop_sequences.rs` to keep the
  stop-sequence compatibility predicate separate from request structs.
- Updated chat and legacy completion unsupported-field detection to accept only
  missing stop sequences or an empty `stop` array.
- Added route coverage for fixture-backed chat and legacy completion requests
  with `stop: []`.

## Red Test

```sh
cargo test -p ferrite-server empty_stop_array -- --nocapture
```

Failed before the implementation with:

```text
unsupported completion field(s): stop
unsupported chat completion field(s): stop
```

## Validation

```sh
cargo test -p ferrite-server empty_stop_array -- --nocapture
cargo test -p ferrite-server openai::schema::stop_sequences -- --nocapture
```

Both commands passed after the implementation.

## Limits

This slice does not implement stop-string matching, generated-text truncation,
token-level stop detection, or streaming stop behavior. Non-empty stop strings
and non-empty stop arrays remain unsupported because they would change
generation semantics.
