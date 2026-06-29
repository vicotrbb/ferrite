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

## Explicit Default Temperature Follow-Up

Tree state:

- Commit before change: `fd3f81b`

Ferrite already accepted omitted `temperature` and deterministic
`temperature: 0` as local no-ops. Some OpenAI-compatible clients send the
default sampling value explicitly as `temperature: 1`; rejecting that value makes
otherwise local-compatible requests fail even though the client is not asking
for a non-default sampling behavior relative to the OpenAI request shape.

This slice keeps Ferrite deterministic and does not add general sampling. It
only accepts `temperature: 1` as another no-op compatibility value while
continuing to reject behavior-changing values such as `temperature: 0.2`.

Red checks:

```sh
cargo test -p ferrite-server openai::chat_option_tests::chat_endpoint_accepts_openai_default_temperature -- --nocapture
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_accepts_openai_default_temperature -- --nocapture
```

Observed failures before implementation:

- `chat_endpoint_accepts_openai_default_temperature` returned `400` with
  `unsupported chat completion field(s): temperature`.
- `completions_endpoint_accepts_openai_default_temperature` returned `400` with
  `unsupported completion field(s): temperature`.

Green focused checks:

```sh
cargo test -p ferrite-server openai::chat_option_tests::chat_endpoint_accepts_openai_default_temperature -- --nocapture
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_accepts_openai_default_temperature -- --nocapture
cargo test -p ferrite-server openai::unsupported_tests::chat_endpoint_rejects_sampling_parameters -- --nocapture
```

Observed result:

- `chat_endpoint_accepts_openai_default_temperature`: 1 passed.
- `completions_endpoint_accepts_openai_default_temperature`: 1 passed.
- `chat_endpoint_rejects_sampling_parameters`: 1 passed.

The compatibility boundary remains narrow: `temperature: 0`, `temperature: 1`,
and omitted `temperature` are local no-ops; non-default sampling requests still
return OpenAI-shaped unsupported-field errors.
