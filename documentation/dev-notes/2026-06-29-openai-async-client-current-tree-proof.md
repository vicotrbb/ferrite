# OpenAI Async Client Current-Tree Proof

Date: 2026-06-29

## Scope

This note records a fresh current-tree rerun of Ferrite's `async-openai`
compatibility tests. These tests configure the standard client with a Ferrite
base URL of `http://<addr>/v1` and exercise the OpenAI-compatible server through
client APIs rather than raw HTTP helpers.

Covered surfaces:

- model list and model retrieve;
- legacy completion create;
- legacy completion SSE streaming;
- chat completion create;
- chat completion SSE streaming;
- bearer-token auth through the client's OpenAI API key configuration.

## Verification

Commands:

```sh
cargo test -p ferrite-server --test openai_client_catalog -- --nocapture
cargo test -p ferrite-server --test openai_client_completions -- --nocapture
cargo test -p ferrite-server --test openai_client_chat -- --nocapture
```

Observed results:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

## Result

The current tree remains compatible with the `async-openai` client for the
local-serving subset Ferrite currently targets: model catalog, legacy
completions, chat completions, streaming, and bearer-token auth.

## Limits

These are fixture-backed client tests. They prove client parsing and request
routing through a standard OpenAI-compatible Rust client, but they do not load a
real GGUF model. Real-model HTTP serving is covered separately by the Tier 0 and
Tier 1 current-tree proof notes.
