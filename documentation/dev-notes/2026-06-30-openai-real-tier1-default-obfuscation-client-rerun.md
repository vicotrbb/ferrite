# OpenAI real Tier 1 default obfuscation client rerun

## Context

Ferrite now emits OpenAI streaming `obfuscation` fields by default unless
`stream_options.include_obfuscation` is explicitly false. After aligning the
HTTP route defaults and the reusable stream contexts, the real Tier 1
`async-openai` client path needed a current-tree rerun.

The local workspace has the required Tier 1 Qwen2.5 0.5B Q4_K_M GGUF artifact:

- `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`

## Slice

Rerun the ignored real-model `async-openai` Tier 1 integration suite
sequentially. The suite covers:

- `POST /v1/completions`
- streaming `POST /v1/completions`
- `POST /v1/chat/completions`
- streaming `POST /v1/chat/completions`

The streaming helpers send `stream_options.include_obfuscation: null`, so this
rerun exercises Ferrite's current default-obfuscation behavior through a
standard OpenAI-compatible Rust client.

## Validation

Executed:

- `cargo test -p ferrite-server --test openai_client_real_tier1 -- --ignored --nocapture --test-threads=1`

Result:

- 4 passed, 0 failed, finished in 65.67s.

This proves that the current tree still serves real Tier 1 Qwen2.5 0.5B Q4_K_M
through the tested `async-openai` create and stream paths after default
stream-obfuscation compatibility changes. It does not prove broader OpenAI API
parity, non-Rust client behavior, larger-model default-obfuscation reruns, or
production multi-client concurrency.
