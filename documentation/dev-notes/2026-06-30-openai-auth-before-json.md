# OpenAI Auth Before JSON Parsing

## Context

Ferrite's OpenAI-compatible generation handlers performed bearer-token checks
inside the handler body. Axum request extractors run before the handler, so a
malformed unauthenticated JSON request to a protected generation endpoint could
return request validation errors before authentication errors.

## Change

The generation endpoints now use an authenticated JSON extractor that checks
the configured bearer token before parsing the JSON request body. Catalog,
fallback, method, CORS, and health behavior remain on their existing auth
paths.

## Verification

Run the focused regression:

```sh
cargo test -p ferrite-server --lib openai::auth_tests::protected_generation_routes_authenticate_before_json_extraction -- --nocapture
```
