# OpenAI Model Not Found Param Error Shape

## Context

Ferrite's OpenAI-compatible endpoints returned `model_not_found` for requests
that named an unloaded model, but the structured error body did not identify
the failed parameter. That made the unknown-model branch less consistent with
other OpenAI-style request validation errors.

## Change

`model_not_found` responses now include `error.param` set to `model` while
preserving the existing HTTP 404 status and `model_not_found` error code.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-server returns_model_not_found_for_unknown_model -- --nocapture
```
