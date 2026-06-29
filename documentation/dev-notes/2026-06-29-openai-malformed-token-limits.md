# OpenAI Malformed Token Limits

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat and legacy completion endpoints now return
explicit token-limit field errors when clients provide malformed generation
limit values.

This covers:

- `max_completion_tokens` on `POST /v1/chat/completions`
- `max_tokens` on `POST /v1/completions`

## Red

Added route tests for string token-limit values:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Initial failures:

```text
chat_endpoint_rejects_malformed_max_completion_tokens
Failed to deserialize the JSON body into the target type

completion_endpoint_rejects_malformed_max_tokens
Failed to deserialize the JSON body into the target type
```

Both failures showed that request-body deserialization happened before
Ferrite's normal OpenAI-shaped unsupported-field validation could name the
specific field.

## Implementation

- Added `crates/ferrite-server/src/openai/schema/token_limit.rs`.
- Parsed token-limit request fields through a small schema type that preserves
  valid unsigned integer limits and records malformed values.
- Wired chat `max_tokens` and `max_completion_tokens` to the helper while
  preserving the existing effective-limit precedence.
- Wired legacy completion `max_tokens` to the same helper.
- Added unsupported-field reporting for malformed token-limit fields.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Result:

```text
51 passed; 0 failed; 0 ignored
```

## Limits

This slice does not change Ferrite's token-limit policy. Zero and above-hard
integer values still flow to the existing token-limit normalization errors.
This only ensures malformed request field types produce OpenAI-shaped field
errors instead of generic JSON body deserialization failures.
