# OpenAI Finish Reason Length

## Scope

This slice makes Ferrite's OpenAI-compatible response shapes distinguish token
budget exhaustion from an actual stop condition. Ordinary one-token fixture
requests now report `finish_reason: "length"` when generation reaches the
requested token limit without EOS or a configured stop sequence.

This is server response-shape compatibility evidence. It does not claim new
real-model coverage or broader OpenAI API parity.

## Implementation

`GeneratedText` now carries a `GenerationFinishReason` from the runtime. The
runtime marks generation as:

- `Stop` when callback-controlled stopping or tokenizer EOS ends generation;
- `Length` when the requested token budget is exhausted.

The OpenAI schema layer maps those runtime reasons to OpenAI finish-reason
strings. The mapping stays at the server schema boundary; runtime types remain
generic and do not depend on HTTP-specific response structs.

Stop-sequence trimming still overrides the finish reason to `Stop`, preserving
the behavior proven in the stop-sequence regression tests.

## Red-Green Evidence

Red command:

```sh
cargo test -p ferrite-server --lib openai::response_shape_tests:: -- --nocapture
```

Observed failures before the implementation:

```text
assertion `left == right` failed
  left: String("stop")
 right: "length"
Error: "expected length event"
```

The four failing tests were:

- `completions_endpoint_returns_openai_choice_shape`
- `completions_stream_endpoint_returns_openai_choice_shape`
- `chat_endpoint_returns_openai_message_shape`
- `chat_stream_endpoint_returns_openai_choice_shape`

Green command:

```sh
cargo test -p ferrite-server --lib openai::response_shape_tests:: -- --nocapture
```

Observed result after the implementation:

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 248 filtered out; finished in 0.00s
```

Stop-sequence regression command:

```sh
cargo test -p ferrite-server --lib openai::stop_sequences_tests:: -- --nocapture
```

Observed result:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 246 filtered out; finished in 0.02s
```

## Result

Legacy completions, chat completions, and their SSE streaming variants now
report `finish_reason: "length"` when the local fixture model exhausts the
requested generation budget. Stop-sequence paths continue to report
`finish_reason: "stop"`.
