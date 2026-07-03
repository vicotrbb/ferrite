# Long-Chat Capsule-Only Placement

## Context

The structured-continuity-anchor probes showed that placing a compact state
capsule in the follow-up user message preserved `state_anchor=7291` through a
64-token generated-context window, with lower prompt cost when the capsule was
shortened to `state_anchor=7291`.

The next falsification slice needs to know whether retained generated prose is
still useful or mostly adds prompt cost. The existing long-chat gate could
decorate retained assistant prose, or decorate the follow-up instruction, but
could not omit retained prose while still carrying the proof-only state capsule.

## Change

Added a proof-only long-chat gate placement:

```text
--generated-context-state-capsule-placement assistant-context-only
```

On generated follow-up turns, this placement uses only:

```text
Ferrite state capsule:
<capsule>
```

as the assistant context. It does not include the previous generated assistant
response. The seed turn is unchanged, and default serving behavior is
unchanged.

## Validation

Red test before implementation:

```text
cargo test -p ferrite-server --test long_chat_gate can_use_state_capsule_as_generated_follow_up_context_without_retained_prose -- --nocapture
```

Initial failure:

```text
--generated-context-state-capsule-placement must be assistant-context or follow-up
```

Post-change checks:

```text
cargo test -p ferrite-server --test long_chat_gate can_use_state_capsule_as_generated_follow_up_context_without_retained_prose -- --nocapture
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

The targeted test passed, and the full long-chat gate integration test file
reported `56 passed`.

## Limits

This is a proof-harness option only. It does not prove that capsule-only
context preserves semantic continuity on a real model, and it does not change
Ferrite's OpenAI-compatible HTTP serving policy.
