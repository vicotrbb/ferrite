# OpenAI Completion String Array Prompts

Date: 2026-06-29

## Summary

Ferrite's OpenAI-compatible legacy completion endpoint now accepts `prompt` as
either a string or an array of strings.

The supported local-serving shapes are:

```json
{"prompt":"hello"}
```

```json
{"prompt":["hello","world"]}
```

For non-streaming requests, Ferrite evaluates each text prompt sequentially
under the existing single inference permit and returns one completion choice per
prompt with matching choice indexes. Usage counts are aggregated across the
generated completions.

Token prompt forms, such as `prompt: [1, 2, 3]` or `prompt: [[1, 2, 3]]`,
remain unsupported because the current OpenAI server path is scoped to local
text generation.

The official OpenAI Completions API reference describes `prompt` as a string,
array of strings, array of tokens, or array of token arrays.

Reference:

- <https://developers.openai.com/api/reference/resources/completions/methods/create/>

## Implementation Notes

- Added `openai::schema::completion_prompt` as a focused parser for text prompt
  shapes.
- Added multi-generation response construction with per-choice indexes and
  aggregate usage.
- Added `generate_texts` to run several prompts through the loaded model under
  one acquired inference permit.
- Kept streaming completions single-prompt only, because multi-prompt streaming
  requires indexed stream chunk behavior that should be designed separately.

## Verification

Red test:

```sh
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_accepts_array_of_string_prompts -- --nocapture
```

Initial result before implementation:

- The request returned `400` with `Failed to deserialize the JSON body into the
  target type`.

Focused final checks:

```sh
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_accepts_array_of_string_prompts -- --nocapture
cargo test -p ferrite-server openai::schema::completion_prompt -- --nocapture
```

Observed result:

- `completions_endpoint_accepts_array_of_string_prompts`: 1 passed.
- `openai::schema::completion_prompt`: 3 passed.
