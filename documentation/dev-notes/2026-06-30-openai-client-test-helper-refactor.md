# OpenAI Client Test Helper Refactor

Date: 2026-06-30

## Scope

This slice extracted the repeated `async-openai` client construction used by
Ferrite's OpenAI-compatible integration tests into
`crates/ferrite-server/tests/support/openai_client.rs`.

The helper builds a standard `async-openai` client with Ferrite's local `/v1`
base URL and a caller-supplied API key. Existing fixture-backed client tests and
the real Tier 0 client proof now use this shared helper.

This is a test-organization refactor only. It does not change server routes,
request or response schemas, authentication behavior, streaming behavior, or
inference execution.

## Baseline

Before the refactor:

```sh
cargo test -p ferrite-server --test openai_client_catalog -- --nocapture
cargo test -p ferrite-server --test openai_client_completions -- --nocapture
cargo test -p ferrite-server --test openai_client_chat -- --nocapture
cargo test -p ferrite-server --test openai_client_real_tier0 -- --ignored --nocapture
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.73s
```

## Verification

After the refactor:

```sh
cargo test -p ferrite-server --test openai_client_catalog -- --nocapture
cargo test -p ferrite-server --test openai_client_completions -- --nocapture
cargo test -p ferrite-server --test openai_client_chat -- --nocapture
cargo test -p ferrite-server --test openai_client_real_tier0 -- --ignored --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.56s
```

The formatting, diff hygiene, and clippy commands exited successfully.
