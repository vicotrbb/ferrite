# OpenAI Real Tier 0 HTTP Chat Proof

Date: 2026-06-28

## Summary

Ferrite's opt-in real-model HTTP integration coverage now includes chat
completions and chat streaming with a real Tier 0 GGUF model.

The proof starts a live Axum server with
`target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`, sends raw HTTP/1.1 requests
to `POST /v1/chat/completions`, and verifies deterministic one-token output
through both non-streaming and streaming OpenAI-compatible chat paths.

## Expected Output Probe

Ferrite's current chat renderer turns a single user message into:

```text
user: hello world
assistant:
```

CLI probe:

```sh
target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt $'user: hello world\nassistant: ' --generate-tokens 1
```

Observed output:

```text
prompt_token_ids=4093,42,33662,905,198,520,9531,42,216
generated_token_ids=198
generated_text=
```

The generated token is a newline, so the HTTP chat tests assert `"\n"`.

## Implementation Notes

- Added `live_http_server_chats_with_real_tier0_model` to
  `crates/ferrite-server/tests/openai_real_model_http.rs`.
- Added `live_http_server_streams_chat_with_real_tier0_model` to
  `crates/ferrite-server/tests/openai_real_model_http.rs`.
- The tests remain ignored by default because they require a local GGUF
  artifact and load the real model.
- No production server code changed.

## Verification

Explicit real-model proof:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 4 ignored real-model HTTP tests passed when explicitly enabled.
- The target took about 96.49s because each ignored test starts a live server
  and loads the Tier 0 GGUF model.

The new chat tests verified:

- non-streaming `POST /v1/chat/completions` returns `chat.completion`.
- non-streaming chat content is `"\n"`.
- usage counts for the rendered chat prompt are 9 prompt tokens, 1 completion
  token, and 10 total tokens.
- streaming `POST /v1/chat/completions` returns `text/event-stream`.
- streaming chat emits a `chat.completion.chunk` with `delta.content` set to
  `"\n"`.
- streaming chat ends with `data: [DONE]`.

Default server verification before the test commit:

```sh
cargo fmt --all
cargo test -p ferrite-server -- --nocapture
cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed result:

- `cargo test -p ferrite-server -- --nocapture`: 48 unit tests passed,
  7 `openai_client` integration tests passed, 6 `openai_http` integration
  tests passed, and 4 real-model HTTP tests were ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves real Tier 0 GGUF execution through non-streaming and streaming
OpenAI-compatible chat paths. It does not prove conversation quality, richer
chat templating, multi-turn behavior, or Tier 1+ server behavior.
