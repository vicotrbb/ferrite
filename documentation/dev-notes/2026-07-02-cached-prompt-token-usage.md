# Cached Prompt Token Usage Metadata

Date: 2026-07-02

## Slice

This slice adds no-behavior-change cached-prompt-token metadata plumbing for
OpenAI-compatible usage accounting.

Ferrite still does not perform cross-request prompt-cache lookup or K/V reuse.
Generated output now has an explicit cached-prompt-token count that defaults to
zero, and OpenAI response usage reads from that field instead of hard-coding
`cached_tokens = 0`.

## Implementation

- Added `GeneratedText::cached_prompt_tokens`.
- Added `GeneratedText::with_cached_prompt_tokens`, which rejects cached-token
  counts greater than the full prompt-token count.
- Updated OpenAI usage serialization to report cached tokens from
  `GeneratedText`.
- Updated multi-prompt completion usage to sum cached prompt tokens across all
  generated choices.

## Red Test

Initial tests were added before implementation in:

- `crates/ferrite-server/src/runtime.rs`
- `crates/ferrite-server/src/openai/schema/usage.rs`

The first compile failed as expected:

```text
error[E0599]: no method named `with_cached_prompt_tokens` found for struct `runtime::GeneratedText`
```

## Validation

Focused checks:

```sh
cargo test -p ferrite-server --lib generated_text_records_cached_prompt_tokens -- --nocapture
cargo test -p ferrite-server --lib generated_text_rejects_cached_prompt_tokens_above_prompt_tokens -- --nocapture
cargo test -p ferrite-server --lib openai::schema::usage::tests -- --nocapture
```

Broader checks:

```sh
cargo test -p ferrite-server --lib
cargo fmt --all -- --check
git diff --check
```

Results:

- Focused metadata tests passed.
- `cargo test -p ferrite-server --lib`: 355 passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Environment Note

An initial isolated-target run failed before reaching the test assertion because
the local filesystem had about 1.3 GiB free and Cargo could not write build
artifacts. Removing the temporary `target/codex-token-prefix` and
`target/codex-cache-metadata` directories restored enough space for the shared
target validation.

## Limits

This slice does not create nonzero cached-token values on real request paths.
It only makes the accounting path ready for future exact-prefix K/V reuse.
