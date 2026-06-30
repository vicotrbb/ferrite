# OpenAI null completion echo proof

## Context

Ferrite's OpenAI-compatible legacy completions endpoint supports `echo: false`
as a neutral option and `echo: true` as prompt echo behavior. Explicit JSON
`null` values are common in generated OpenAI-compatible request bodies, so
`echo: null` needs a regression guard even though it should behave like an
omitted option.

## Slice

Add focused fixture coverage for:

- `POST /v1/completions` with `echo: null`

## Validation

Executed:

- `cargo test -p ferrite-server --lib completions_endpoint_accepts_null_echo -- --nocapture`

Result:

- 1 passed, 0 failed.

The test passed before any production-code change, confirming the current
serde-backed request schema already treats explicit `echo: null` as absent.
This slice records that compatibility behavior and guards against regression.
It does not add new completion logprobs behavior or broaden real-model proof.
