# OpenAI Method Not Allowed Errors

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible router now returns an OpenAI-shaped JSON error body
when a client uses an unsupported HTTP method on a known endpoint.

This keeps wrong-method client probes from receiving Axum's default empty `405`
response body, which is hard for OpenAI-style clients to interpret consistently.

## Red

Added `completions_endpoint_returns_openai_error_for_wrong_method` with a `GET`
request to `POST /v1/completions`'s path.

Command:

```sh
cargo test -p ferrite-server completions_endpoint_returns_openai_error_for_wrong_method -- --nocapture
```

Result:

```text
Error: Error("EOF while parsing a value", line: 1, column: 0)
test openai::routes_tests::completions_endpoint_returns_openai_error_for_wrong_method ... FAILED
```

The failure confirmed the route returned a `405` with an empty body rather than
the desired OpenAI error envelope.

## Implementation

- Added `OpenAiHttpError::method_not_allowed()`.
- Registered `Router::method_not_allowed_fallback(method_not_allowed)` in the
  OpenAI router.
- Kept the behavior centralized so individual endpoint handlers do not need
  duplicate wrong-method branches.

The response uses:

- HTTP status: `405 Method Not Allowed`
- error type: `invalid_request_error`
- error code: `method_not_allowed`

## Green

Command:

```sh
cargo test -p ferrite-server completions_endpoint_returns_openai_error_for_wrong_method -- --nocapture
```

Result:

```text
test openai::routes_tests::completions_endpoint_returns_openai_error_for_wrong_method ... ok
```

## Limits

This slice only changes wrong-method handling for known OpenAI-compatible
routes. It does not add a custom JSON body for completely unknown paths.
