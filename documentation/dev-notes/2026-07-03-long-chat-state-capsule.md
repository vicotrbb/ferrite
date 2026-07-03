# Long Chat State Capsule Gate

Date: 2026-07-03 UTC, 2026-07-02 local time

## Change

Added proof-only `ferrite-openai-long-chat-gate` support for:

```text
--generated-context-state-capsule TEXT
```

When configured, the long-chat gate decorates generated follow-up assistant
contexts with a compact model-facing state capsule:

```text
Ferrite state capsule:
<TEXT>

Generated assistant context:
<windowed generated context>
```

The seed assistant context is left unchanged. This keeps the option scoped to
generated follow-up turns and avoids changing Ferrite's OpenAI-compatible HTTP
server defaults.

## Why

The 256-token continuity-anchor proof showed a split result:

- `FERRITE-CONTINUITY-7291` failed at turn 2 in 32-token and 64-token windows.
- `7291` passed four turns in 32-token and 64-token windows.

That suggests continuity quality depends on the shape of retained state, not
only on the generated-context token budget. The state capsule flag makes that
theory testable without changing public serving behavior.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate state_capsule -- --nocapture
error[E0599]: no method named `generated_context_state_capsule` found for struct `LongChatGateConfig`
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate state_capsule -- --nocapture
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 41 filtered out

cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Remaining Proof

This commit only adds the proof harness support. It does not prove that state
capsules improve model continuity. The next gate should run 32-token and
64-token windows against a real model with key/value or JSON capsule anchors,
then compare the result with the prior numeric-anchor and full-marker probes.
