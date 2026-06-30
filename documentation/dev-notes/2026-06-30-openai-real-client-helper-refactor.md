# OpenAI Real Client Helper Refactor

Date: 2026-06-30

## Scope

This slice moved duplicated `async-openai` real-model client assertions from the
Tier 0 and Tier 1 proof files into shared helpers in
`crates/ferrite-server/tests/support/openai_client.rs`.

The model-specific proof files now keep only:

- the local model artifact path;
- the model id;
- the expected one-token completion and chat outputs;
- test-local serialization where the model proof needs it.

This is a test-organization refactor only. It does not change Ferrite server
routes, request schemas, response schemas, streaming behavior, auth behavior, or
inference execution.

## Baseline

Before the refactor:

```sh
cargo test -p ferrite-server --test openai_client_real_tier0 -- --ignored --nocapture
cargo test -p ferrite-server --test openai_client_real_tier1 -- --ignored --nocapture
```

Observed results:

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.78s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 71.06s
```

## Verification

After the refactor:

```sh
cargo test -p ferrite-server --test openai_client_real_tier0 -- --ignored --nocapture
cargo test -p ferrite-server --test openai_client_real_tier1 -- --ignored --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed results:

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 45.21s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 90.19s
```

The formatting, diff hygiene, and clippy commands exited successfully.

## Result

The real OpenAI-client proof surface is now easier to extend to additional
models without copying request construction, stream consumption, or response
shape assertions into every model-specific file.
