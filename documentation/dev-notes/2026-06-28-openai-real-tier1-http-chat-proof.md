# OpenAI Real Tier 1 HTTP Chat Proof

Date: 2026-06-28

## Summary

Ferrite's opt-in real Tier 1 HTTP integration coverage now includes
OpenAI-compatible chat completions and streaming chat chunks.

The proof starts a live Axum server with
`target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`, sends raw HTTP/1.1 requests
to `POST /v1/chat/completions`, and verifies OpenAI-shaped responses for the
deterministic first generated token from the rendered chat prompt.

## Prompt Probe

The expected first token was measured with the CLI before adding the HTTP tests:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt $'user: hello world\nassistant: ' --generate-tokens 1
```

Observed output:

```text
prompt_token_ids=872,25,23811,1879,198,77091,25,220
next_token_id=108386
next_token=ä½łå¥½
generated_token_ids=108386
generated_stopped_on_eos=false
generated_text=你好
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=221184
```

## Implementation Notes

- Added `live_http_server_chats_with_real_tier1_model` to
  `crates/ferrite-server/tests/openai_real_tier1_http.rs`.
- Added `live_http_server_streams_chat_with_real_tier1_model` to the same
  opt-in integration target.
- Both tests remain ignored by default because they require a local Tier 1 model
  artifact and load a 379 MB GGUF file.
- No production server code changed.

## Verification

Explicit Tier 1 HTTP proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 4 ignored Tier 1 real-model HTTP tests passed when explicitly enabled.
- Rust test harness time for the target: about 81.74s.

The new chat tests verified:

- HTTP status `200 OK`.
- non-streaming `chat.completion` response shape.
- streaming `chat.completion.chunk` Server-Sent Events response shape.
- generated chat content `"你好"`.
- prompt token count `8`, completion token count `1`, total token count `9`.
- `data: [DONE]` streaming terminator.

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
  tests passed, 4 real Tier 0 HTTP tests were ignored by default, and 4 real
  Tier 1 HTTP tests were ignored by default.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Boundary

This proves real Tier 1 Qwen2.5-0.5B Q4_K_M execution through both
non-streaming and streaming OpenAI-compatible chat completions HTTP paths. It
does not prove Tier 1 server throughput, concurrent real-model serving, or
broader Tier 1 model coverage.
