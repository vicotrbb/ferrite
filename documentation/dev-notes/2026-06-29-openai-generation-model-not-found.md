# OpenAI Generation Model Not Found

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible generation endpoints now return the same
OpenAI-shaped `model_not_found` error used by `GET /v1/models/{model}` when a
request names a model id that is not the loaded local model.

This applies to:

- `POST /v1/completions`
- `POST /v1/chat/completions`

The change keeps the local server's model catalog behavior consistent across
model retrieval and generation without changing inference execution.

## Red

The new route tests first failed because generation model mismatches returned a
generic `400` invalid request instead of `404` with `code:
model_not_found`.

```sh
cargo test -p ferrite-server completions_endpoint_returns_model_not_found_for_unknown_model -- --nocapture
cargo test -p ferrite-server chat_endpoint_returns_model_not_found_for_unknown_model -- --nocapture
```

Observed failures:

```text
left: 400
right: 404
```

## Green

The shared generation model check now reuses `OpenAiHttpError::model_not_found`
instead of constructing a generic invalid-request response.

Focused checks:

```sh
cargo test -p ferrite-server completions_endpoint_returns_model_not_found_for_unknown_model -- --nocapture
cargo test -p ferrite-server chat_endpoint_returns_model_not_found_for_unknown_model -- --nocapture
```

Observed result:

- both focused tests passed.

## Interpretation

This slice does not add multiple loaded models or dynamic model routing. It
only makes a requested-model mismatch explicit and consistent for OpenAI-style
clients before inference is attempted.
