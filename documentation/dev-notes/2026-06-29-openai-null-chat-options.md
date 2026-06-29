# OpenAI Null Chat Options

Ferrite now accepts explicit JSON `null` for documented Chat Completions
request options that the local engine does not implement.

## Why

OpenAI's Chat Completions API documents request fields such as `audio`,
`moderation`, `prediction`, `verbosity`, and `web_search_options`. Some
OpenAI-compatible clients serialize unset optional fields as JSON `null`.
Ferrite should treat those null values as absent while still rejecting any
non-null value that would require unsupported audio output, moderation, predicted
output, verbosity control, or web search behavior.

Reference:
https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Changes

- Added typed nullable request fields for:
  - `audio`
  - `moderation`
  - `prediction`
  - `verbosity`
  - `web_search_options`
- Kept non-null values unsupported through `ChatCompletionRequest::unsupported_fields()`.
- Added a route-level fixture test proving these fields are accepted when they
  are explicitly null.

## TDD Evidence

Red test:

```bash
cargo test -p ferrite-server chat_endpoint_accepts_null_optional_openai_options -- --nocapture
```

Expected failure before implementation:

```text
unsupported chat completion field(s): audio, moderation, prediction, verbosity, web_search_options
```

Focused green check:

```bash
cargo test -p ferrite-server chat_endpoint_accepts_null_optional_openai_options -- --nocapture
```
