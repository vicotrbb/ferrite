# Long Chat Mixed Cache Key Lanes

Date: 2026-07-03

## Goal

Add proof-harness coverage for mixed prompt-cache keys so long-chat gates can
exercise cache isolation across multiple logical tenants or threads.

## Context

`documentation/theories/2026-07-03-semantic-capsule-cache-scaling.md` identifies
mixed cache keys as the next proof risk after the 256, 512, and 1024-token
capsule-only semantic-cache gates. The previous proof established that a stable
single lane can reuse cached generated follow-up context. It did not prove that
multiple keys stay isolated when exercised in the same gate.

## Changes

- Added `--prompt-cache-keys KEY[,KEY...]` to
  `ferrite-openai-long-chat-gate`.
- Kept `--prompt-cache-key` as the single-lane option and rejected combining it
  with `--prompt-cache-keys`.
- Expanded long-chat scenarios by prompt-cache key when multiple keys are
  provided.
- Passed each scenario lane's key into the OpenAI-compatible throughput client.
- Isolated generated assistant context by model, token length, and prompt-cache
  key so one key cannot seed another key's follow-up prompt.
- Isolated generated-context identity summaries by prompt-cache key.
- Included mixed-key plan and scenario output in the gate report.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
error: unexpected argument '--prompt-cache-keys' found
test result: FAILED. 56 passed; 4 failed
```

Green test evidence:

```text
cargo fmt --check

cargo test -p ferrite-server --lib
test result: ok. 392 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo clippy -p ferrite-server --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 47.02s
```

## Limits

This is a proof-harness capability, not a runtime cache optimization. It does
not yet prove real-model mixed-key behavior, concurrent clients, cache eviction,
or a no-cache 1024-token baseline.

## Next Proof

Run a bounded real-model mixed-key gate on staging with the same Qwen2.5-1.5B
Q8_0 model, semantic capsule settings, and two prompt-cache keys. Start at 256
tokens before widening to 512 or 1024.
