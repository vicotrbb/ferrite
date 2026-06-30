# OpenAI Completion Echo

Date: 2026-06-29

## Scope

Support `echo: true` for non-streaming OpenAI-compatible legacy completions by
prefixing each returned completion choice text with its original prompt.

This is a response-shape compatibility slice for `/v1/completions`. It does
not change inference, tokenization, sampling, prompt execution, chat
completions, or streaming completion chunks.

Source reference:

- https://developers.openai.com/api/reference/resources/completions/methods/create/

## Rationale

The OpenAI Completions reference documents `echo` as returning the prompt in
addition to the generated completion. Ferrite already keeps the request prompt
at the OpenAI server layer, so this can be implemented without leaking
HTTP-specific types into the inference core.

Streaming `echo: true` remained unsupported in this slice. It was added later
in `documentation/dev-notes/2026-06-30-openai-streaming-completion-echo.md`.

## Change

- Added fixture coverage for non-streaming `echo: true`.
- Added explicit rejection coverage for `stream: true` with `echo: true`.
- Added `CompletionRequest::echo`.
- Added `CompletionResponse::from_prompt_generations` to prefix prompt text
  when echoing is requested.
- Routed non-streaming legacy completion responses through the prompt-aware
  response constructor.

## Red Tests

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests -- --nocapture
```

Observed result before implementation:

- `completions_endpoint_echoes_prompt_when_requested` failed with HTTP `400`
  and `error.param = "echo"`.
- The rest of the suite passed.

## Validation

Post-implementation validation:

```sh
cargo test -p ferrite-server --lib openai::completion_option_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib openai::response_shape_tests -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
```

Observed result:

- `openai::completion_option_tests`: 9 passed.
- `openai::completion_unsupported_tests`: 10 passed.
- `openai::response_shape_tests`: 2 passed.
- Formatting check passed.
- Whitespace check passed.
- `ferrite-server` clippy passed with warnings denied.

## Limits

This slice did not support streaming echo when it landed. Token-level logprobs
for echoed prompt tokens and ignored real-model GGUF HTTP suites remained out
of scope.
