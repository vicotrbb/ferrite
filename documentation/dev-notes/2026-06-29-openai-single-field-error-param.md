# OpenAI Single-Field Error Param

Date: 2026-06-29

## Scope

Ferrite now fills the OpenAI error `param` field for unsupported request errors
when exactly one request field is rejected.

## Red

Added focused assertions to existing unsupported-field route tests:

- `chat_endpoint_rejects_sampling_parameters` expects
  `error.param == "temperature"`.
- `completion_endpoint_rejects_logprobs_request` expects
  `error.param == "logprobs"`.

Initial focused run:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

The new assertions failed with:

```text
left: Null
right: "temperature"

left: Null
right: "logprobs"
```

## Implementation

- Added an invalid-request constructor that accepts an error parameter.
- Kept the existing unsupported-field error messages.
- Added the parameter only when unsupported-field validation reports exactly
  one field.
- Left multi-field unsupported request errors with `param: null` because there
  is no single precise parameter to report.

## Green

Focused verification after implementation:

```text
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Result:

```text
55 passed; 0 failed; 0 ignored
```

## Limits

This slice covers unsupported-field validation. It does not yet add `param`
values for every other invalid-request path, such as missing required fields,
token-limit normalization errors, malformed JSON bodies, or unknown models.
