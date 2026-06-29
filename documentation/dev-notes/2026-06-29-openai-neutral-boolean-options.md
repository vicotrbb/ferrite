# OpenAI Neutral Boolean Options

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible endpoints now accept a small set of disabled
boolean options that are semantics-neutral for local text generation:

- legacy completions: `echo: false`
- chat completions: `logprobs: false`
- chat completions: `store: false`

Enabled values still return OpenAI-shaped unsupported-field errors because they
would require behavior Ferrite does not currently implement:

- `echo: true` changes the legacy completion response text;
- `logprobs: true` changes the chat response shape and requires token
  log-probability accounting; and
- `store: true` is hosted OpenAI storage behavior, not local inference.

Source references:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Implementation

- Extended `crates/ferrite-server/src/openai/schema/neutral_options.rs` with a
  boolean neutral-option predicate.
- Updated chat unsupported-field detection to allow only `logprobs: false` and
  `store: false`.
- Updated legacy completion unsupported-field detection to allow only
  `echo: false`.
- Added fixture-backed route tests for both endpoint families.

## Red Test

```sh
cargo test -p ferrite-server disabled -- --nocapture
```

Failed before the implementation with:

```text
unsupported completion field(s): echo
unsupported chat completion field(s): logprobs, store
```

## Validation

```sh
cargo test -p ferrite-server disabled -- --nocapture
cargo test -p ferrite-server openai::schema::neutral_options -- --nocapture
```

Both commands passed after the implementation.

## Limits

This slice does not implement echoing prompts, log-probability responses,
hosted storage semantics, or any new sampling behavior.
