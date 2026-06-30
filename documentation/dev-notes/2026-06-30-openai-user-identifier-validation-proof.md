# OpenAI User Identifier Validation Proof

## Summary

Ferrite now has route-level OpenAI-compatible validation coverage for malformed
`user` request identifiers on both generation endpoints:

- `POST /v1/chat/completions`;
- `POST /v1/completions`.

String `user` identifiers were already accepted by route tests, and the schema
helper already rejected non-string values. This slice proves the HTTP boundary
returns an OpenAI-shaped `invalid_request_error` with `error.param` set to
`user` for malformed client payloads.

## Result

No production code change was required. The existing request schema and
unsupported-field error path already produce the desired client-facing error
shape.

## Validation

```text
cargo test -p ferrite-server --lib openai::user_identifier_tests -- --nocapture
```

Result: passed.
