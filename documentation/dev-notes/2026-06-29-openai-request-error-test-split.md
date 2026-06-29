# OpenAI Request Error Test Split

Date: 2026-06-29

## Scope

Split protocol-level OpenAI request error tests out of the large
`routes_tests.rs` module into `request_error_tests.rs`.

This is a test-organization slice only. It does not change OpenAI server
runtime behavior, request schemas, response schemas, or inference execution.

## Rationale

The OpenAI route test module had grown beyond one thousand lines while covering
several distinct concerns: health/catalog behavior, generation routes, streaming
helpers, queue behavior, malformed JSON, wrong methods, and unknown routes.

Moving request-shape and route-protocol errors into a focused module keeps the
OpenAI server tests easier to scan and follows the repository goal of keeping
files focused rather than allowing monster files to grow.

## Moved Tests

- `completions_endpoint_returns_openai_error_for_malformed_json`
- `completions_endpoint_returns_openai_error_for_missing_json_content_type`
- `completions_endpoint_returns_openai_error_for_wrong_method`
- `unknown_openai_route_returns_openai_error_body`

## Verification Plan

Run:

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server openai::request_error_tests -- --nocapture
cargo test -p ferrite-server openai::routes_tests -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
```

## Results

Observed after the split:

- `routes_tests.rs`: 1071 lines.
- `request_error_tests.rs`: 88 lines.
- `unsupported_tests.rs`: still 1199 lines and remains a future split
  candidate.

Commands:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-server openai::request_error_tests -- --nocapture
cargo test -p ferrite-server openai::routes_tests -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
```

Observed results:

- `openai::request_error_tests`: 4 passed, 0 failed.
- `openai::routes_tests`: 44 passed, 0 failed.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace -- --nocapture`: exit 0; `ferrite-server` lib tests
  reported 223 passed, 0 failed.
- Default workspace real GGUF suites remained ignored; this was a test
  organization slice, not a fresh real-model proof.

## Limits

This slice only reduces one OpenAI test-file concern. Other large test modules,
including unsupported-field coverage, may need additional focused splits later.
