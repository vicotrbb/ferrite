# OpenAI No-Tool Options

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat completion endpoint now accepts explicit
no-tool request options:

- `tools: []`
- `tool_choice: "none"`
- `parallel_tool_calls: false`

OpenAI documents `tool_choice: "none"` as the default when no tools are present.
The accepted values in this slice are no-ops for Ferrite's current local text
generation path. Tool definitions, automatic tool choice, required tool choice,
forced tool calls, and enabled parallel tool calls remain unsupported.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/tool_options.rs` to keep
  no-tool compatibility detection separate from the chat request type.
- Updated chat unsupported-field detection to accept only empty tools,
  `tool_choice: "none"`, and `parallel_tool_calls: false`.
- Added a fixture-backed chat route test covering the explicit no-tool
  combination.

## Red Test

```sh
cargo test -p ferrite-server explicit_no_tool_options -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): tools, tool_choice, parallel_tool_calls
```

## Validation

```sh
cargo test -p ferrite-server explicit_no_tool_options -- --nocapture
cargo test -p ferrite-server openai::schema::tool_options -- --nocapture
```

Both commands passed after implementation.

## Limits

This slice does not implement tool calling, function calling, tool-call response
objects, tool-result messages, forced tool selection, `tool_choice: "auto"`,
`tool_choice: "required"`, or `parallel_tool_calls: true`.
