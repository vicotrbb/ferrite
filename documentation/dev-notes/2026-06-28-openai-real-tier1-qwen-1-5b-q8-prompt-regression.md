# 2026-06-28 OpenAI Real Tier 1 Qwen 1.5B Q8 Prompt Regression

## Scope

This slice adds ignored real-model HTTP regressions for six fixed
Qwen2.5-1.5B Q8_0 prompt profiles through `POST /v1/completions` and
`POST /v1/chat/completions`, including SSE streaming responses for both
endpoint families.

The test is isolated in
`crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_prompts.rs` so prompt
coverage stays separate from endpoint-shape and queue-order regressions.

## Test Shape

- Model: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model id: `qwen2.5-1.5b-q8_0`
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

Each legacy completion and non-streaming chat response asserts HTTP `200`, the
OpenAI object shape, the configured model id, the decoded first-token
text/content, and prompt/completion usage counts.

Each streaming completion response asserts HTTP `200`, the `text/event-stream`
content type, OpenAI `text_completion` objects, the configured model id, the
decoded first-token text chunk, exactly one terminal stop chunk with empty
text, and the `[DONE]` terminator.

Each streaming chat response asserts HTTP `200`, the `text/event-stream`
content type, OpenAI `chat.completion.chunk` objects, the configured model id,
the decoded first-token delta content, exactly one terminal stop chunk without
delta content, and the `[DONE]` terminator.

## Expected First Token

| Prompt | Completion prompt tokens | Completion text | Completion stream text | Chat prompt tokens | Chat content | Chat stream content |
| --- | ---: | --- | --- | ---: | --- | --- |
| `hello world` | 2 | `\n` | `\n` | 8 | `你好` | `你好` |
| `The capital of France is` | 5 | ` Paris` | ` Paris` | 11 | ` Paris` | ` Paris` |
| `Once upon a time` | 4 | `,` | `,` | 10 | `1` | `1` |
| `Rust is a systems programming language` | 7 | ` that` | ` that` | 12 | `你说` | `你说` |
| `Machine learning models can` | 4 | ` be` | ` be` | 10 | `1` | `1` |
| `The recipe calls for` | 4 | ` ` | ` ` | 10 | `2` | `2` |

## Debugging Note

The first run failed on `The recipe calls for`:

```text
assertion `left == right` failed
  left: String(" ")
 right: " 2"
```

Root cause: the documented six-token reference continuation starts with token
`220`, decoded as a single space; token `17`, decoded as `2`, is the second
generated token. The regression was corrected to assert one generated token.

## Verification

```sh
cargo fmt --check
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_streaming_prompts live_http_server_streams_qwen_1_5b_q8_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_streams_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Result:

```text
test live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts ... ignored, requires local Qwen2.5-1.5B Q8_0 GGUF model artifact
test live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts ... ignored, requires local Qwen2.5-1.5B Q8_0 GGUF model artifact
test result: ok. 0 passed; 0 failed; 2 ignored

test live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 296.39s

test live_http_server_streams_qwen_1_5b_q8_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 290.79s

test live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 626.61s

test live_http_server_streams_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 611.76s
```

## Interpretation

This narrows the broader model/prompt behavior gap for Ferrite's
OpenAI-compatible server by proving six deterministic Qwen2.5-1.5B Q8_0
reference prompts through the HTTP legacy completion path, the streaming legacy
completion path, and the non-streaming and streaming chat completion paths.

It does not prove broad prompt behavior, longer completions through HTTP,
x86_64 HTTP behavior, or server throughput.

## Current-Tree Rerun After Stream Role Chunk Compatibility

Tree state:

- Commit: `dcff88a`
- Model artifact: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- File size: 1.8 GB

The first current-tree full prompt rerun exposed a stale streaming-chat
assertion:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts -- --ignored --nocapture
```

Observed failure:

```text
thread 'live_http_server_streams_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts' panicked at crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_prompts.rs:241:5:
assertion `left == right` failed: unexpected generated chat stream chunks
  left: ["", "你好"]
 right: ["你好"]

test live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts ... ok
test live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts ... ok
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 782.26s
```

Root cause: the prompt helper counted Ferrite's OpenAI-style initial assistant
role chunk as generated content because that chunk has `finish_reason: null` and
`delta.content: ""`. The current response-shape contract expects this role
chunk. The prompt helper now collects generated text only from non-terminal
chunks where `delta.role` is absent, preserving the role chunk while excluding
it from the generated-content comparison.

Focused rerun of the failed path:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_streams_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Observed result:

```text
test live_http_server_streams_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 697.73s
```

Current-tree reruns of the other prompt paths:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts -- --ignored --nocapture
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_prompts live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts -- --ignored --nocapture
```

Observed results:

```text
test live_http_server_matches_qwen_1_5b_q8_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 425.72s

test live_http_server_matches_qwen_1_5b_q8_chat_first_tokens_for_reference_prompts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 611.64s
```

This rerun confirms the Qwen2.5-1.5B Q8_0 six-prompt legacy completion,
non-streaming chat, and streaming chat paths at the current tree after aligning
the streaming-chat assertion with the OpenAI-compatible role-chunk contract.
