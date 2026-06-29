# OpenAI Neutral Sampling Options

Date: 2026-06-29

## Summary

Ferrite's OpenAI-compatible chat and legacy completion endpoints now accept a
small set of deterministic no-op sampling options commonly emitted by OpenAI
clients:

- `temperature: 0`
- `top_p: 1`
- `n: 1`
- `presence_penalty: 0`
- `frequency_penalty: 0`

This improves local-client compatibility without pretending Ferrite supports
general sampling, multi-choice generation, penalties, or stop-sequence
semantics. Behavior-changing values still return OpenAI-shaped
`invalid_request_error` responses.

## Implementation Notes

- Added `openai::schema::neutral_options` as a focused helper for numeric
  no-op request values.
- Updated chat and completion unsupported-field detection to permit only the
  neutral values above.
- Added fixture-backed route tests proving both endpoints still generate when
  neutral sampling options are present.

## Verification

Red test:

```sh
cargo test -p ferrite-server neutral_sampling_options -- --nocapture
```

Initial result before implementation:

- `completions_endpoint_accepts_neutral_sampling_options` returned `400` with
  `unsupported completion field(s): temperature, top_p, n, presence_penalty,
  frequency_penalty`.
- `chat_endpoint_accepts_neutral_sampling_options` returned `400` with
  `unsupported chat completion field(s): temperature, top_p, n,
  presence_penalty, frequency_penalty`.

Focused final checks:

```sh
cargo test -p ferrite-server neutral_sampling_options -- --nocapture
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
cargo test -p ferrite-server openai::schema::neutral_options -- --nocapture
```

Observed result:

- `neutral_sampling_options`: 2 passed.
- `openai::unsupported_tests`: 8 passed.
- `openai::schema::neutral_options`: 3 passed.
