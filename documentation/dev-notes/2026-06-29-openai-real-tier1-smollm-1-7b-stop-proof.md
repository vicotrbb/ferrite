# OpenAI Real Tier 1 SmolLM2-1.7B Q4_K_M Stop Proof

## Scope

This slice extends real Tier 1 OpenAI-compatible HTTP `stop` sequence evidence
to SmolLM2-1.7B-Instruct Q4_K_M. It uses the existing known one-token
`hello world` completion and chat shapes so the requested stop sequence fully
trims the visible generated token.

This is compatibility evidence for the HTTP server boundary. It does not claim
broader prompt coverage, longer generations, x86_64 stop behavior, or full
Tier 1 HTTP completion.

## Code Organization

The new proof is isolated in
`crates/ferrite-server/tests/openai_real_tier1_smollm_1_7b_stop.rs` and reuses
the shared stop-response assertions from
`crates/ferrite-server/tests/support/stop_sequences.rs`.

## Local Artifact

Tree state:

- Commit before this slice: `a81da16`
- Model artifact:
  `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- File size: 1.0 GB

## Stop Regression

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_stop live_http_server_applies_stop_sequences_with_smollm_1_7b_q4_model -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_stop_sequences_with_smollm_1_7b_q4_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 195.54s
```

This proves supported OpenAI `stop` sequence trimming for SmolLM2-1.7B Q4_K_M
across these four one-token HTTP shapes:

- legacy completion with `stop: "\""` trimming the known `hello world`
  completion token;
- streaming legacy completion with `stop: "\""` trimming the same token;
- chat completion with `stop: "1"` trimming the known `hello world` chat token;
  and
- streaming chat completion with `stop: "1"` trimming the same chat token.

The non-streaming responses preserve generated-token usage accounting while
returning empty visible text/content. The streaming responses suppress visible
text/content chunks and still emit a terminal `finish_reason: "stop"` event and
`data: [DONE]`.
