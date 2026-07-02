# OpenAI Proof Client Prompt Cache Key

Date: 2026-07-02

## Goal

Make Ferrite's local proof clients able to send OpenAI-compatible
`prompt_cache_key` metadata through `POST /v1/chat/completions`.

This supports the long-chat prefix-cache proof path and makes comparison with
external clients such as `llama-benchy` cleaner, because `llama-benchy` can send
the same field via `--extra-body prompt_cache_key=...`.

## Context

The server already accepts `prompt_cache_key` on chat requests and the
experimental prefix-cache flag can enable exact-prefix reuse.

The missing local proof-client piece was request construction:

- `ferrite-openai-throughput` could not set `prompt_cache_key`;
- `ferrite-openai-long-chat-gate` could not pass a cache namespace into the
  throughput client.

## Changes

- Added `--prompt-cache-key KEY` to `ThroughputClientConfig`.
- Reject the flag for legacy `/v1/completions`; Ferrite currently treats it as
  a chat-completion field.
- Serialize `"prompt_cache_key":"..."` in chat-completion request bodies.
- Added `--prompt-cache-key KEY` to `LongChatGateConfig`.
- Passed the long-chat cache key through generated throughput-client args.

## Red Tests

The new focused tests first failed for the expected missing APIs:

```text
error[E0599]: no method named `prompt_cache_key` found for struct `throughput_client::config::ThroughputClientConfig`
error[E0599]: no method named `prompt_cache_key` found for struct `LongChatGateConfig`
```

## Validation

Focused checks:

```sh
CARGO_TARGET_DIR=target/codex-prompt-cache-throughput cargo test -p ferrite-server throughput_client::tests::builds_openai_compatible_chat_prompt_cache_key_request_body -- --nocapture
CARGO_TARGET_DIR=target/codex-prompt-cache-long-chat cargo test -p ferrite-server --test long_chat_gate passes_prompt_cache_key_to_long_chat_throughput_config -- --nocapture
```

Results:

- throughput prompt-cache-key request-body test: 1 passed.
- long-chat pass-through test: 1 passed.

Related module checks:

```sh
CARGO_TARGET_DIR=target/codex-prompt-cache-throughput cargo test -p ferrite-server --lib throughput_client::tests -- --nocapture
CARGO_TARGET_DIR=target/codex-prompt-cache-long-chat cargo test -p ferrite-server --test long_chat_gate -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Results:

- throughput client tests: 46 passed.
- long-chat gate tests: 25 passed.
- formatting check: passed.
- whitespace check: passed.

## Results

Ferrite's local proof clients can now send an explicit chat prompt-cache
namespace, including during the long-chat gate.

This is harness wiring only. It did not run a real model, did not run
`llama-benchy`, and did not prove real-model cache speedup or memory behavior.

## Follow-Ups

- Run one Ferrite long-chat smoke with `--prompt-cache-key` and
  `--experimental-prefix-cache`.
- Run one `llama-benchy` baseline smoke against the same server and model.
- Compare Ferrite's streamed timing/RSS output with `llama-benchy` JSON before
  promoting `llama-benchy` to a regular benchmark gate.
