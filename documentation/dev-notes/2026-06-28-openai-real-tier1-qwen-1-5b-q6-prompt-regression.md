# 2026-06-28 OpenAI Real Tier 1 Qwen 1.5B Q6 Prompt Regression

## Scope

This slice adds ignored real-model HTTP regressions for six fixed
Qwen2.5-1.5B Q6_K prompt profiles through `POST /v1/completions` and
`POST /v1/chat/completions`, including non-streaming and SSE streaming
responses for both endpoint families.

The non-streaming and chat tests are isolated in
`crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_q6_prompts.rs`, while
the streaming legacy completion test is isolated in
`crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_q6_streaming_prompts.rs`
so Q6_K prompt coverage remains separate from Q8_0 prompt coverage,
endpoint-shape coverage, and queue-order coverage.

## Test Shape

- Model: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model id: `qwen2.5-1.5b-q6_k`
- Endpoints:
  - `POST /v1/completions`
  - `POST /v1/chat/completions`
- Generation limit: 1 token per prompt
- Prompts:
  - `hello world`
  - `The capital of France is`
  - `Once upon a time`
  - `Rust is a systems programming language`
  - `Machine learning models can`
  - `The recipe calls for`

Each legacy completion response asserts HTTP `200`, the OpenAI
`text_completion` object shape, the configured model id, the decoded
first-token text, and prompt/completion usage counts.

Each streaming legacy completion response asserts HTTP `200`, the
`text/event-stream` content type, OpenAI `text_completion` objects, the
configured model id, the decoded first-token text chunk, exactly one terminal
stop chunk with empty text, and the `[DONE]` terminator.

Each chat completion response asserts HTTP `200`, the OpenAI
`chat.completion` object shape, the configured model id, the decoded
first-token message content, and prompt/completion usage counts.

Each streaming chat completion response asserts HTTP `200`, the
`text/event-stream` content type, OpenAI `chat.completion.chunk` objects, the
configured model id, the decoded first-token delta content, exactly one
terminal stop chunk without delta content, and the `[DONE]` terminator.

| Prompt | Completion prompt tokens | Completion text | Completion stream text | Chat prompt tokens | Chat content | Chat stream content |
| --- | ---: | --- | --- | ---: | --- | --- |
| `hello world` | 2 | `\n` | `\n` | 8 | `你好` | `你好` |
| `The capital of France is` | 5 | ` Paris` | ` Paris` | 11 | ` Paris` | ` Paris` |
| `Once upon a time` | 4 | `,` | `,` | 10 | `一次` | `一次` |
| `Rust is a systems programming language` | 7 | ` that` | ` that` | 12 | `你说` | `你说` |
| `Machine learning models can` | 4 | ` be` | ` be` | 10 | `1` | `1` |
| `The recipe calls for` | 4 | ` ` | ` ` | 10 | `2` | `2` |

## Verification

```sh
cargo fmt --check
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_matches_qwen_1_5b_q6_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_streaming_prompts live_http_server_streams_qwen_1_5b_q6_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_matches_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_streams_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Result:

```text
test live_http_server_matches_qwen_1_5b_q6_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 286.58s

test live_http_server_streams_qwen_1_5b_q6_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 287.77s

test live_http_server_matches_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 701.29s

test live_http_server_streams_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 692.57s
```

## Interpretation

This narrows the broader model/prompt behavior gap for Ferrite's
OpenAI-compatible server by proving six deterministic Qwen2.5-1.5B Q6_K
reference prompts through the HTTP legacy completion path, streaming legacy
completion path, non-streaming chat completion path, and streaming chat
completion path.

It does not prove broad prompt behavior, longer completions through HTTP,
x86_64 HTTP behavior, queue fairness, or server throughput.

## Current-Tree Stream Role Chunk Rerun

Tree state:

- Commit: `ce05be1`
- Model artifact: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- File size: 1.4 GB

The current-tree focused rerun reproduced the same stale stream assertion shape
seen in the Q8_0 prompt proof:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_streams_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Observed failure:

```text
assertion `left == right` failed: unexpected generated chat stream chunks
  left: ["", "你好"]
 right: ["你好"]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 95.47s
```

Root cause: the helper counted Ferrite's OpenAI-style initial assistant role
chunk as generated content because that role chunk has `finish_reason: null`
and `delta.content: ""`. The current stream response contract expects the role
chunk, so the helper now compares generated content only from non-terminal
chunks where `delta.role` is absent.

Green rerun:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_q6_prompts live_http_server_streams_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_streams_qwen_1_5b_q6_chat_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 751.91s
```

This confirms the Qwen2.5-1.5B Q6_K streaming-chat six-prompt path at the
current tree after aligning the assertion with the OpenAI-compatible role-chunk
contract. The non-streaming Q6_K and legacy streaming Q6_K paths were not rerun
in this current-tree slice.
