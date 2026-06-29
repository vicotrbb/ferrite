# OpenAI Token Limit Error Param

Date: 2026-06-29

## Scope

Return field-specific `error.param` values when a generation request exceeds
Ferrite's configured hard token limit.

This is an OpenAI-compatible HTTP validation slice. It does not change model
loading, tokenization, generation, streaming, or configured token-limit values.

## Rationale

Malformed token-limit request fields already return explicit unsupported-field
errors such as `max_tokens` or `max_completion_tokens`. Hard-limit rejections
happen later in route validation, after schema parsing, and previously lost the
originating request field. OpenAI-compatible clients can use `error.param` to
highlight the invalid field, so route-level hard-limit errors should preserve
that parameter as well.

## Change

- Added focused token-limit route tests in `openai::token_limit_tests`.
- Preserved the token-limit field source on chat requests:
  - `max_tokens` when the legacy chat field is used.
  - `max_completion_tokens` when the modern chat field is used.
- Preserved `max_tokens` as the field source on legacy completion requests.
- Returned `invalid_request_error` with field-specific `error.param` for
  configured hard-limit rejections.
- Formatted requested token-limit error messages with the actual OpenAI request
  field name so `max_completion_tokens` errors do not mention only the legacy
  `max_tokens` field.

## Verification

RED command:

```sh
cargo test -p ferrite-server openai::token_limit_tests -- --nocapture
```

Observed result before the route change:

- `chat_endpoint_reports_max_completion_tokens_param_when_hard_limit_is_exceeded`
  failed because `error.param` was `Null`.
- `completion_endpoint_reports_max_tokens_param_when_hard_limit_is_exceeded`
  failed because `error.param` was `Null`.

GREEN command:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::token_limit_tests -- --nocapture
```

Observed result:

- `openai::token_limit_tests`: 2 passed, 0 failed.

Final slice gates:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
```

Observed results:

- Formatting check passed.
- Whitespace check passed.
- Workspace clippy passed with `-D warnings`.
- Workspace tests passed; `ferrite-server` reported 225 library tests passed,
  including the two new token-limit tests.
- Ignored real-model GGUF HTTP suites remained ignored by the default workspace
  test command.

Follow-up RED command:

```sh
cargo test -p ferrite-server openai::token_limit_tests -- --nocapture
```

Observed result before message-field formatting:

- `chat_endpoint_reports_max_completion_tokens_param_when_hard_limit_is_exceeded`
  failed because the message was `max_tokens must be less than or equal to 2`.

Follow-up GREEN command:

```sh
cargo fmt --all
cargo test -p ferrite-server openai::token_limit_tests -- --nocapture
```

Observed result:

- `openai::token_limit_tests`: 2 passed, 0 failed.

Follow-up final gates:

```sh
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
```

Observed results:

- Formatting check passed.
- Whitespace check passed.
- Workspace clippy passed with `-D warnings`.
- Workspace tests passed; `ferrite-server` reported 225 library tests passed.
- Ignored real-model GGUF HTTP suites remained ignored by the default workspace
  test command.

## Limits

This slice covers fixture HTTP validation only. It does not rerun ignored
real-model GGUF HTTP suites.
