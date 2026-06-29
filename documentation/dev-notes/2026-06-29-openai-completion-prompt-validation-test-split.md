# OpenAI Completion Prompt Validation Test Split

Date: 2026-06-29

## Context

`crates/ferrite-server/src/openai/completion_unsupported_tests.rs` was carrying
both unsupported completion-option tests and prompt-shape validation tests.
Keeping those cases together made the OpenAI compatibility test surface less
focused as the local server grew.

## Change

- Added `completion_prompt_validation_tests.rs` for completion prompt-shape
  failures: missing prompt, null prompt, object prompt, token prompt array, and
  token prompt array batch.
- Added `post_completion_json` to `test_support.rs` so completion route tests
  can share the same JSON helper style already used by chat tests.
- Left `completion_unsupported_tests.rs` focused on unsupported options,
  malformed non-prompt fields, missing/null model, and unknown fields.

## Validation

Baseline before the split:

```sh
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
cargo test -p ferrite-server --lib prompt -- --nocapture
```

After the split:

```sh
cargo test -p ferrite-server --lib openai::completion_prompt_validation_tests -- --nocapture
cargo test -p ferrite-server --lib openai::completion_unsupported_tests -- --nocapture
```

Results:

- `completion_prompt_validation_tests`: 5 passed.
- `completion_unsupported_tests`: 8 passed.
