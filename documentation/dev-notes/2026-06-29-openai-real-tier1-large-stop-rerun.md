# OpenAI Real Tier 1 Larger Stop Rerun

Date: 2026-06-29

## Scope

This slice reran the larger local Tier 1 OpenAI-compatible HTTP stop-sequence
proofs after the streaming stop-filter flush fix. The goal was to verify that
the server still applies configured stop sequences across non-streaming and
streaming completion/chat paths for the larger local Tier 1 artifacts.

No production code changed in this slice.

## Local Artifacts

The rerun used the default local artifact paths required by the integration
tests:

- `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`

## Qwen2.5-1.5B Q8_0

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http live_http_server_applies_stop_sequences_with_qwen_1_5b_q8_model -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_stop_sequences_with_qwen_1_5b_q8_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 244.93s
```

## Qwen2.5-1.5B Q6_K

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_stop -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_stop_sequences_with_qwen_1_5b_q6_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 204.11s
```

## SmolLM2-1.7B Q4_K_M

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_stop -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_stop_sequences_with_smollm_1_7b_q4_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 161.28s
```

## Result

The OpenAI-compatible HTTP server still applies supported stop sequences for
the larger local Tier 1 GGUF artifacts across:

- `POST /v1/completions`;
- streamed `POST /v1/completions`;
- `POST /v1/chat/completions`;
- streamed `POST /v1/chat/completions`.

This is fresh larger-artifact proof after the streaming stop-filter flush fix.
It does not prove Tier 2+ models, broad prompt quality, long-context behavior,
or full Tier 1 completion.
