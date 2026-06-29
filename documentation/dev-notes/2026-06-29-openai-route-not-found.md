# OpenAI Route Not Found Errors

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible router now returns an OpenAI-shaped JSON error body
for unknown paths instead of Axum's default empty `404` response.

This improves local OpenAI-client behavior when clients probe unsupported
OpenAI APIs such as `/v1/responses` while Ferrite still only supports the
focused local text-generation subset from ADR 0008.

## Red

Added `unknown_openai_route_returns_openai_error_body` with a `GET` request to
`/v1/responses`.

Command:

```sh
cargo test -p ferrite-server unknown_openai_route_returns_openai_error_body -- --nocapture
```

Result:

```text
Error: Error("EOF while parsing a value", line: 1, column: 0)
test openai::routes_tests::unknown_openai_route_returns_openai_error_body ... FAILED
```

The failure confirmed the unknown route returned an empty framework body instead
of an OpenAI error envelope.

## Implementation

- Added `OpenAiHttpError::route_not_found(path)`.
- Added an OpenAI router fallback that extracts `OriginalUri` and includes the
  requested path in the error message.
- Kept this separate from the method-not-allowed fallback so known-path method
  errors still return `405 method_not_allowed`.

The response uses:

- HTTP status: `404 Not Found`
- error type: `invalid_request_error`
- error code: `not_found`

## Green

Commands:

```sh
cargo test -p ferrite-server unknown_openai_route_returns_openai_error_body -- --nocapture
cargo test -p ferrite-server openai::routes_tests -- --nocapture
```

Results:

```text
test openai::routes_tests::unknown_openai_route_returns_openai_error_body ... ok
```

```text
test result: ok. 48 passed; 0 failed; 0 ignored
```

## Limits

This slice does not add support for the Responses API or any other unsupported
OpenAI API family. It only ensures unsupported route probes receive a structured
OpenAI-style error body.
