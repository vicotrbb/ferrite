# OpenAI Empty Logit Bias

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat and legacy completion endpoints now accept
`logit_bias: {}` as a local no-op.

OpenAI documents `logit_bias` as a token-bias map. Ferrite still does not
implement token-specific logit biasing, but an empty map changes no logits and
is safe to treat the same as a missing or `null` value.

References:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create
- https://developers.openai.com/api/reference/resources/completions/methods/create

## Red

The focused HTTP fixture tests first tried:

```sh
cargo test -p ferrite-server accepts_empty_logit_bias -- --nocapture
```

Both endpoints failed before implementation with OpenAI-shaped unsupported-field
errors:

```text
unsupported completion field(s): logit_bias
unsupported chat completion field(s): logit_bias
```

## Green

Changes:

- Added a focused `logit_bias` schema helper.
- Chat and legacy completion requests now treat missing, `null`, and empty
  object `logit_bias` values as neutral.
- Non-empty and malformed `logit_bias` values remain unsupported until Ferrite
  implements token-level logit biasing.

Verification:

```sh
cargo test -p ferrite-server accepts_empty_logit_bias -- --nocapture
cargo test -p ferrite-server logit_bias -- --nocapture
```

Both focused filters passed after implementation.

## Boundary

This slice does not implement OpenAI logit bias semantics, token-id mapping, or
sampling-time bias application. It only accepts the documented empty-map no-op
shape so local OpenAI clients that send default empty option maps continue to
work.
