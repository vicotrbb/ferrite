# OpenAI Parallel Tool Calls Without Tools

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now accepts
`parallel_tool_calls` as a neutral boolean only when no tools are configured.
This covers clients that serialize the OpenAI default option even when they are
not using tool calling.

OpenAI documents `parallel_tool_calls` as an optional boolean for parallel
function calling during tool use. With `tools` missing or `tools: []`, the
boolean does not request any local behavior from Ferrite.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Made chat unsupported-field detection validate `parallel_tool_calls` with the
  request's `tools` value.
- Accepted boolean `parallel_tool_calls` values only when `tools` is missing or
  empty.
- Preserved rejection for malformed values and non-empty tool definitions.

## Red Test

```sh
cargo test -p ferrite-server parallel_tool_calls_without_tools -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): parallel_tool_calls
```

## Validation

```sh
cargo test -p ferrite-server parallel_tool_calls_without_tools -- --nocapture
cargo test -p ferrite-server tool_options -- --nocapture
```

Both commands passed after implementation.

## Limits

This slice does not implement tool calling, function calling, parallel tool
execution, `tool_choice: "auto"`, `tool_choice: "required"`, or forced tool
selection.
