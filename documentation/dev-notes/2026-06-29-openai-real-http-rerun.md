# OpenAI Real HTTP Rerun

Date: 2026-06-29

## Scope

Ferrite's ignored real-model OpenAI HTTP integration tests were explicitly
rerun against local GGUF artifacts after the compatibility work on chat seed
and model-not-found generation errors.

This was a proof rerun only. No production code changed in this slice.

A second proof rerun was performed at commit `af6ea08` after the
method-not-allowed OpenAI error-envelope slice. That rerun used the same default
local artifact paths and is recorded below.

## Local Artifacts

The rerun used the default model paths required by the integration tests:

- Tier 0: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Tier 1: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

Both files were present locally before the tests were run.

## Tier 0 OpenAI HTTP Proof

Command:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 4 ignored tests were explicitly enabled.
- 4 passed, 0 failed, 0 ignored.
- Rust test harness duration: 21.15s.

Covered behavior:

- real Tier 0 legacy completion over `POST /v1/completions`;
- real Tier 0 legacy completion streaming over OpenAI-style SSE;
- real Tier 0 chat completion over `POST /v1/chat/completions`;
- real Tier 0 chat completion streaming over OpenAI-style SSE.

## Tier 1 OpenAI HTTP Proof

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 6 ignored tests were explicitly enabled.
- 6 passed, 0 failed, 0 ignored.
- Rust test harness duration: 128.86s.

Covered behavior:

- real Tier 1 legacy completion over `POST /v1/completions`;
- real Tier 1 legacy completion streaming over OpenAI-style SSE;
- real Tier 1 chat completion over `POST /v1/chat/completions`;
- real Tier 1 chat completion streaming over OpenAI-style SSE;
- real Tier 1 concurrent request rejection with the default zero wait timeout;
- real Tier 1 queued concurrent request completion with configured wait time.

## Interpretation

This rerun proves that the current OpenAI-compatible HTTP server still drives
real Tier 0 and Tier 1 GGUF models through the local Ferrite runtime for the
covered one-token deterministic requests.

It does not prove Tier 2+ model support, long-context behavior, conversation
quality, or the larger Qwen2.5-1.5B and SmolLM2-1.7B suites.

## Post-Method-Error Rerun

Tree state:

- Commit: `af6ea08`
- Local artifact paths present:
  - `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
  - `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

Tier 0 command:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 4 ignored tests were explicitly enabled.
- 4 passed, 0 failed, 0 ignored.
- Rust test harness duration: 20.13s.

Tier 1 command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 6 ignored tests were explicitly enabled.
- 6 passed, 0 failed, 0 ignored.
- Rust test harness duration: 133.16s.

This confirms a fresh real-model OpenAI HTTP run after the latest route-error
compatibility work. It still does not prove the larger Tier 1 prompt-regression
suites or any Tier 2+ models.

## Post-Validation-And-Auth Rerun

Tree state:

- Commit: `0f5bcdf`
- Local artifact paths present:
  - `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
  - `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

This rerun followed the OpenAI compatibility commits for:

- request-error test organization;
- unknown OpenAI route errors;
- token-limit `error.param` and message alignment;
- case-insensitive bearer auth scheme parsing.

Tier 0 command:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 4 ignored tests were explicitly enabled.
- 4 passed, 0 failed, 0 ignored.
- Rust test harness duration: 19.14s.

Tier 1 command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 6 ignored tests were explicitly enabled.
- 6 passed, 0 failed, 0 ignored.
- Rust test harness duration: 125.69s.

Covered behavior:

- real Tier 0 SmolLM2-135M Q4_K_M legacy completion, streaming legacy
  completion, chat completion, and streaming chat completion;
- real Tier 1 Qwen2.5-0.5B Q4_K_M legacy completion, streaming legacy
  completion, chat completion, streaming chat completion, default
  backpressure, and configured wait-queue completion.

This confirms that the current OpenAI-compatible HTTP server still drives real
Tier 0 and Tier 1 GGUF models through Ferrite-owned loading, tokenization,
generation, and streaming paths after the latest local-serving compatibility
work. It still does not prove Tier 2+ models, larger Tier 1 prompt-regression
suites, long-context behavior, or broad conversation quality.

## Post-Unique-ID Rerun

Tree state:

- Commit: `d0741d9`
- Local artifact paths present:
  - `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
  - `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

This rerun followed the OpenAI compatibility commit that changed chat,
completion, and stream response IDs from timestamp-only values to unique
process-local IDs.

Tier 0 command:

```sh
cargo test -p ferrite-server --test openai_real_model_http -- --ignored --nocapture
```

Observed result:

- 4 ignored tests were explicitly enabled.
- 4 passed, 0 failed, 0 ignored.
- Rust test harness duration: 19.76s.

Tier 1 command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_http -- --ignored --nocapture
```

Observed result:

- 6 ignored tests were explicitly enabled.
- 6 passed, 0 failed, 0 ignored.
- Rust test harness duration: 124.48s.

Covered behavior:

- real Tier 0 SmolLM2-135M Q4_K_M legacy completion, streaming legacy
  completion, chat completion, and streaming chat completion;
- real Tier 1 Qwen2.5-0.5B Q4_K_M legacy completion, streaming legacy
  completion, chat completion, streaming chat completion, default
  backpressure, and configured wait-queue completion.

This confirms that the OpenAI-compatible HTTP server still drives real Tier 0
and default Tier 1 GGUF models through Ferrite-owned loading, tokenization,
generation, and SSE streaming after the response-ID compatibility change.
