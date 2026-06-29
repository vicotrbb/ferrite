# OpenAI Stream Stop Filter Flush

## Scope

This slice fixes streaming behavior when a request configures a supported
OpenAI `stop` sequence that does not match generated output. The streaming stop
filter previously held all pending text until generation finished, then emitted
one combined visible chunk.

This is fixture-backed server behavior. It does not add new real-model stop
coverage or broader OpenAI API parity.

## Implementation

`StopSequenceFilter` now retains only the pending suffix that could still become
a configured stop-sequence prefix. Any prefix that cannot participate in a
future stop match is flushed immediately as a visible stream chunk.

The retention check walks UTF-8 character boundaries in each configured stop
sequence, so multibyte stop prefixes remain safe. The streaming endpoints keep
their existing behavior when a stop sequence does match: visible text before
the stop is emitted, generation stops, and the terminal chunk reports
`finish_reason: "stop"`.

## Red-Green Evidence

Red command:

```sh
cargo test -p ferrite-server --lib openai::stop_sequences_tests:: -- --nocapture
```

Observed failures before the implementation:

```text
assertion `left == right` failed: data: ... "text":"winnerwinner" ...
  left: 1
 right: 2
```

The failing tests were:

- `completions_stream_endpoint_flushes_chunks_when_stop_sequence_does_not_match`
- `chat_stream_endpoint_flushes_chunks_when_stop_sequence_does_not_match`

Both failures proved that a two-token streaming request with `stop: "zzz"`
emitted one combined `winnerwinner` chunk instead of two generated token chunks.

Green command:

```sh
cargo test -p ferrite-server --lib openai::stop_sequences_tests:: -- --nocapture
```

Observed result after the implementation:

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 247 filtered out; finished in 0.00s
```

Helper regression:

```sh
cargo test -p ferrite-server --lib openai::generation::tests:: -- --nocapture
```

Observed result:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 254 filtered out; finished in 0.00s
```

## Result

Streaming legacy completions and streaming chat completions now continue to
emit visible chunks promptly when a configured stop sequence does not match.
The terminal chunk still reports `finish_reason: "length"` for token-budget
exhaustion, and stop-match paths continue to report `finish_reason: "stop"`.
