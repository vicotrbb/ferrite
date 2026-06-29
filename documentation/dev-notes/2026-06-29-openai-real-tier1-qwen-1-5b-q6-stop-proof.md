# OpenAI Real Tier 1 Qwen2.5-1.5B Q6_K Stop Proof

## Scope

This slice extends real Tier 1 OpenAI-compatible HTTP `stop` sequence evidence
from Qwen2.5-1.5B Q8_0 to Qwen2.5-1.5B Q6_K. It uses the same known
single-token `hello world` shapes so the expected generated token is fully
trimmed by the requested stop sequence.

This is compatibility evidence for the HTTP server boundary. It does not claim
broader prompt coverage, longer generations, SmolLM2-1.7B stop behavior,
x86_64 stop behavior, or full Tier 1 HTTP completion.

## Code Organization

The reusable stop-response assertions now live in
`crates/ferrite-server/tests/support/stop_sequences.rs`, keeping the existing
Qwen2.5-1.5B Q8_0 test file focused and avoiding another large integration test
file. The new Q6_K proof is isolated in
`crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_q6_stop.rs`.

## Local Artifacts

Tree state:

- Commit before this slice: `eee2c3b`
- Q6_K model artifact:
  `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Q6_K file size: 1.4 GB
- Q8_0 model artifact:
  `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Q8_0 file size: 1.8 GB

## Q6_K Stop Regression

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_stop live_http_server_applies_stop_sequences_with_qwen_1_5b_q6_model -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_stop_sequences_with_qwen_1_5b_q6_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 298.08s
```

This proves supported OpenAI `stop` sequence trimming for Qwen2.5-1.5B Q6_K
across these four one-token HTTP shapes:

- legacy completion with `stop: "\n"`;
- streaming legacy completion with `stop: "\n"`;
- chat completion with `stop: "你"`; and
- streaming chat completion with `stop: "你"`.

The non-streaming responses preserve generated-token usage accounting while
returning empty visible text/content. The streaming responses suppress visible
text/content chunks and still emit a terminal `finish_reason: "stop"` event and
`data: [DONE]`.

## Q8_0 Refactor Rerun

The existing Qwen2.5-1.5B Q8_0 stop regression was rerun after moving shared
stop-response assertions into test support.

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http live_http_server_applies_stop_sequences_with_qwen_1_5b_q8_model -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_applies_stop_sequences_with_qwen_1_5b_q8_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 242.01s
```

This keeps the previously recorded Q8_0 evidence current after the test
organization change.
