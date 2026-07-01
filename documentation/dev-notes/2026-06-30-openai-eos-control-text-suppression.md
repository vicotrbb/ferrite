# OpenAI EOS Control Text Suppression

## Context

The SmolLM2 OpenAI-compatible EOS proof showed that Ferrite stops naturally on
tokenizer EOS and reports OpenAI `finish_reason: "stop"`, but it also surfaced
the raw tokenizer control text as visible output:

```text
<|im_end|>
```

That is useful runtime evidence, but it is not the desired OpenAI-compatible
HTTP boundary behavior. EOS should count toward completion usage and terminate
generation, while the control marker should not be returned as assistant text.

## Change

`InferenceEngine::generate_with_token_callback` now checks the generated token
id against `tokenizer.ggml.eos_token_id` before passing decoded text to the
visible token callback. When EOS is generated:

- the generated token is still counted in `completion_tokens`;
- the finish reason is still `GenerationFinishReason::Stop`;
- visible response text omits the EOS control token;
- streaming responses emit the terminal stop chunk and `[DONE]` without a
  preceding content chunk for the EOS marker.

## Validation

The regression tests first failed with visible `winner` output when fixture
token id `2` was configured as EOS. After the fix:

```sh
cargo test -p ferrite-server --lib suppresses_visible_eos_text -- --nocapture
cargo test -p ferrite-server --lib openai::stop_sequences_tests:: -- --nocapture
cargo test -p ferrite-server --lib runtime::tests -- --nocapture
CARGO_BUILD_JOBS=2 cargo clippy -p ferrite-server --all-targets -- -D warnings
git diff --check
```

Observed results:

- EOS suppression regression: 2 passed.
- `openai::stop_sequences_tests`: 10 passed.
- `runtime::tests`: 2 passed.
- `cargo clippy -p ferrite-server --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Scope

This is a server runtime behavior change. It does not change the separate CLI
generated-token proof path, and it does not claim EOS behavior across every
Tier 1 model yet. The current proof is for the OpenAI-compatible server
boundary and its fixture regression coverage.
