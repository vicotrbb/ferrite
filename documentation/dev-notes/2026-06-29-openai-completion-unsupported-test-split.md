# OpenAI Completion Unsupported Test Split

Date: 2026-06-29

## Scope

Split legacy completion unsupported-field tests out of the large
`unsupported_tests.rs` module into `completion_unsupported_tests.rs`.

This is a test-organization slice only. It does not change OpenAI server
runtime behavior, request validation, response schemas, or inference execution.

## Rationale

`unsupported_tests.rs` mixed chat-specific unsupported-field coverage with
legacy completion unsupported-field coverage and had grown beyond one thousand
lines. Moving completion-specific tests into a focused module keeps the OpenAI
server test suite easier to scan and aligns with the repository preference for
small, focused Rust files.

## Moved Tests

- `completion_endpoint_rejects_multiple_choice_request`
- `completion_endpoint_rejects_logprobs_request`
- `completion_endpoint_rejects_missing_model`
- `completion_endpoint_rejects_null_model`
- `completion_endpoint_rejects_missing_prompt`
- `completion_endpoint_rejects_null_prompt`
- `completion_endpoint_rejects_object_prompt`
- `completion_endpoint_rejects_malformed_seed`
- `completion_endpoint_rejects_malformed_max_tokens`
- `completion_endpoint_rejects_malformed_stream_flag`
- `completion_endpoint_rejects_token_prompt_array`
- `completion_endpoint_rejects_token_prompt_array_batch`
- `completion_endpoint_rejects_unknown_fields`

## Verification

Safety-net command before the split:

```sh
cargo test -p ferrite-server completion_endpoint_rejects -- --nocapture
```

Observed result:

- 14 passed, 0 failed.

Focused commands after the split:

```sh
cargo test -p ferrite-server openai::completion_unsupported_tests -- --nocapture
cargo test -p ferrite-server openai::unsupported_tests -- --nocapture
```

Observed results:

- `openai::completion_unsupported_tests`: 13 passed, 0 failed.
- `openai::unsupported_tests`: 42 passed, 0 failed.

The extra test in the pre-split filtered command came from
`stream_options_tests::completion_endpoint_rejects_stream_options_without_streaming`;
it was not part of this module split.

## Limits

This slice only separates completion unsupported-field coverage. The remaining
chat unsupported-field module is still large enough to merit future smaller
topic-based splits.
