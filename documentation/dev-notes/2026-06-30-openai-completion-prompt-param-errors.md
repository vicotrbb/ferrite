# OpenAI Completion Prompt Param Errors

## Context

Ferrite's OpenAI-compatible completions route rejected empty prompts,
whitespace-only prompts, and streaming requests with multiple prompts, but those
route-level validation errors did not populate `error.param`. The message named
`prompt`, but OpenAI-style clients can handle request failures more precisely
when the structured parameter field is present.

## Change

Completion prompt route validation now sets `error.param` to `prompt` for:

- empty prompt lists;
- whitespace-only text prompts;
- streaming completion requests that do not contain exactly one text prompt.

## Verification

Run the focused regressions:

```sh
cargo test -p ferrite-server completion_endpoint_reports_prompt_param_for_empty_or_whitespace_prompt -- --nocapture
cargo test -p ferrite-server streaming_completion_prompt_validation_runs_before_engine_availability -- --nocapture
```
