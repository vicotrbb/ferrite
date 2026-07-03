# Long Chat State Capsule Placement

Date: 2026-07-03 UTC, 2026-07-02 local time

## Change

Added proof-only placement control for long-chat state capsules:

```text
--generated-context-state-capsule-placement assistant-context|follow-up
```

The default remains `assistant-context`, preserving the existing behavior:

```text
Ferrite state capsule:
<TEXT>

Generated assistant context:
<windowed generated context>
```

The new `follow-up` placement leaves the generated assistant context unchanged
and instead decorates generated follow-up turns:

```text
Ferrite state capsule:
<TEXT>

Follow-up instruction:
<follow-up prompt>
```

This remains scoped to the `ferrite-openai-long-chat-gate` proof harness. It
does not alter Ferrite's OpenAI-compatible HTTP server defaults or request
schemas.

## Why

The first state-capsule real-model probe passed the 32-token generated-context
window but failed the 64-token window at turn 2. That suggests the model may
need stronger capsule placement or less competition between retained assistant
prose and the structured anchor.

The follow-up placement tests whether user-message authority preserves the
anchor more reliably than embedding the capsule inside generated assistant
context.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate state_capsule -- --nocapture
error[E0599]: no method named `generated_context_state_capsule_placement` found for struct `LongChatGateConfig`
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate state_capsule -- --nocapture
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 41 filtered out

cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Next Proof

Run the same x86_64 Qwen2.5-1.5B Q8_0 256-token state-capsule gate with:

```text
--generated-context-state-capsule state_anchor=7291
--generated-context-state-capsule-placement follow-up
```

The first comparison should use the previously failing 64-token window before
expanding back to a 32/64 pair.
