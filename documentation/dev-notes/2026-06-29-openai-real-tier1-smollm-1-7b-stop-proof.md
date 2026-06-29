# OpenAI Real Tier 1 SmolLM2-1.7B Q4_K_M Stop Proof

## Scope

This slice extends real Tier 1 OpenAI-compatible HTTP `stop` sequence evidence
to SmolLM2-1.7B-Instruct Q4_K_M. It uses the existing known one-token
`hello world` completion and chat shapes plus the six established legacy
completion reference prompts so the requested stop sequence fully trims the
visible generated token.

This is compatibility evidence for the HTTP server boundary. It does not claim
longer generations, x86_64 stop behavior, broad chat stop prompt coverage, or
full Tier 1 HTTP completion.

## Code Organization

The one-prompt four-endpoint proof is isolated in
`crates/ferrite-server/tests/openai_real_tier1_smollm_1_7b_stop.rs`. The
six-prompt legacy completion proof lives separately in
`crates/ferrite-server/tests/openai_real_tier1_smollm_1_7b_stop_prompts.rs` so
the real-model prompt matrix does not turn into a large mixed-purpose test
file. Both tests reuse the shared stop-response assertions from
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

## Six-Prompt Legacy Completion Stop Regression

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_smollm_1_7b_stop_prompts live_http_server_applies_completion_stop_sequences_to_smollm_1_7b_q4_reference_prompts -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_completion_stop_sequences_to_smollm_1_7b_q4_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 506.87s
```

This expands SmolLM2-1.7B Q4_K_M real-model `stop` coverage across the six
established legacy completion reference prompts, in both non-streaming and
streaming completion mode:

- `hello world`;
- `The capital of France is`;
- `Once upon a time`;
- `Rust is a systems programming language`;
- `Machine learning models can`; and
- `The recipe calls for`.

Each request uses the already documented one-token completion text as the
requested stop sequence. Non-streaming responses preserve generated-token usage
accounting and return empty visible text. Streaming responses suppress visible
text chunks and still emit a terminal `finish_reason: "stop"` event and
`data: [DONE]`.

The observed pass was captured before the test was split into the focused
`openai_real_tier1_smollm_1_7b_stop_prompts` target; the test body was moved
unchanged so the current rerunnable command above points at the organized file.

## Broad Chat Stop Gap

Broad six-prompt SmolLM2-1.7B chat stop-sequence coverage is still not claimed
by this note. A diagnostic attempt against the first `hello world`
non-streaming chat stop request was interrupted after it remained in the first
request for several minutes on the local scalar path. The existing one-prompt
four-endpoint proof above remains the current SmolLM2 chat stop evidence.
