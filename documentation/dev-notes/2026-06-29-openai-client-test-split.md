# OpenAI Client Test Split

Date: 2026-06-29

## Context

`crates/ferrite-server/tests/openai_client.rs` mixed all `async-openai` client
proofs in one integration target: model catalog, legacy completions,
streaming legacy completions, chat completions, API-key auth, and streaming
chat completions.

Splitting these proofs keeps the OpenAI-compatible client surface easier to
extend without turning one integration test file into a grab bag.

## Change

- Replaced `openai_client.rs` with focused integration targets:
  - `openai_client_catalog.rs`
  - `openai_client_completions.rs`
  - `openai_client_chat.rs`
- Kept the same live fixture-server behavior and the same seven `async-openai`
  client coverage points.
- Updated the README client-proof sentence to name catalog, completions, chat,
  streaming, and bearer-token coverage.

## Validation

Baseline before the split:

```sh
cargo test -p ferrite-server --test openai_client -- --nocapture
```

Result:

- `openai_client`: 7 passed.

After the split:

```sh
cargo test -p ferrite-server --test openai_client_catalog -- --nocapture
cargo test -p ferrite-server --test openai_client_completions -- --nocapture
cargo test -p ferrite-server --test openai_client_chat -- --nocapture
```

Results:

- `openai_client_catalog`: 2 passed.
- `openai_client_completions`: 2 passed.
- `openai_client_chat`: 3 passed.
