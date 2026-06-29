# OpenAI Real HTTP Rerun

Date: 2026-06-29

## Scope

Ferrite's ignored real-model OpenAI HTTP integration tests were explicitly
rerun against local GGUF artifacts after the compatibility work on chat seed
and model-not-found generation errors.

This was a proof rerun only. No production code changed in this slice.

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
