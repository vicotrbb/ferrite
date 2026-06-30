# OpenAI stream context obfuscation default

## Context

Ferrite's HTTP routes now default `stream_options.include_obfuscation` to
enabled unless the client explicitly sends `false`. The public stream schema
contexts re-exported from `ferrite_server::openai::schema` still initialized
their internal `include_obfuscation` flag to `false`, which made direct helper
usage diverge from the route behavior.

## Slice

Default `CompletionStreamContext::new` and `ChatCompletionStreamContext::new`
to emit `obfuscation` fields. Explicit `.with_obfuscation_field(false)` still
disables the field.

## RED

Before the implementation change:

- `cargo test -p ferrite-server --lib openai::schema::completion_stream::tests -- --nocapture`
- `cargo test -p ferrite-server --lib openai::schema::chat_stream::tests -- --nocapture`

failed the new default-obfuscation context tests.

## GREEN

After the implementation change:

- `cargo test -p ferrite-server --lib openai::schema::completion_stream::tests -- --nocapture`
- `cargo test -p ferrite-server --lib openai::schema::chat_stream::tests -- --nocapture`
- `cargo test -p ferrite-server --lib openai::stream_obfuscation_options_tests -- --nocapture`

All commands passed.
