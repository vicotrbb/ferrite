# OpenAI Stop Sequence Early Termination

## Scope

This slice changes supported OpenAI `stop` sequence handling so the server
generation path stops decoding when a stop sequence is observed. Previous
behavior trimmed the visible text after generation, but non-streaming usage
still reflected the full requested token budget.

This is fixture-backed server behavior. It does not claim new real-model stop
coverage, longer-generation model quality, or complete OpenAI API parity.

## Implementation

`InferenceEngine::generate_with_token_callback` now lets the callback return a
generation-control decision. The default `generate` path always continues, and
the OpenAI server path returns `Stop` when its stop-sequence filter observes a
configured stop sequence.

The change keeps the stop-sequence decision in server/OpenAI infrastructure and
does not introduce HTTP-specific types into the inference model or tokenizer
layers.

## Red-Green Evidence

Red command:

```sh
cargo test -p ferrite-server openai::stop_sequences_tests:: -- --nocapture
```

Observed failing assertions before the implementation:

```text
assertion `left == right` failed
  left: Number(3)
 right: 1
```

The failures were from:

- `completions_endpoint_stops_generation_when_stop_sequence_matches`
- `chat_endpoint_stops_generation_when_stop_sequence_matches`

Both failures proved that the response text was already trimmed to `win`, but
`usage.completion_tokens` still counted all three requested generated tokens.

Green command:

```sh
cargo test -p ferrite-server openai::stop_sequences_tests:: -- --nocapture
```

Observed result after the implementation:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 246 filtered out; finished in 0.00s
```

The same filtered package command also enumerated the remaining package test
binaries with zero matching tests and exited successfully.

## Result

For supported string stop sequences, one-token fixture completions and chat
completions now stop generation when the matching token piece is produced.
When the request allows three generated tokens and `stop: "ner"` matches the
first generated `winner` token, the response still returns visible text `win`
and now reports `usage.completion_tokens == 1`.
