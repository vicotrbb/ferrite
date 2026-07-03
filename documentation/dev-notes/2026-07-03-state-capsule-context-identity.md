# State Capsule Context Identity

Date: 2026-07-03

## Goal

Make the long-chat generated-context identity gate sound when a state capsule is
placed in the assistant context.

## Issue

The queue-probe proof first used:

```text
--generated-context-state-capsule 'State capsule: keep answers concise, number the points, and mention CPU, memory, and streaming reliability.'
--generated-context-state-capsule-placement assistant-context
```

All requests completed, but the integrated summary reported:

```text
long_chat_summary_matching_generated_context_identity_links=0
long_chat_summary_all_generated_context_identities_match_previous_response=false
long_chat_summary_run_complete=false
```

That was too strict for the assistant-context capsule design. The previous
generated response was still present, but it was wrapped by the capsule:

```text
Ferrite state capsule:
...

Generated assistant context:
...
```

The summary compared the full rendered assistant-context hash to the previous
generated-response hash, so any wrapper made continuity appear broken.

## Change

- `LongChatScenarioResult` now keeps:
  - full assistant-context identity, for non-disclosing observability;
  - carried generated-context identity, for continuity checks.
- `LongChatAssistantContexts::context_for()` captures the generated-context
  identity before applying any state-capsule wrapper.
- The run summary compares generated-context identity first, then falls back to
  assistant-context identity for older or manually constructed results.
- `format_scenario_result()` prints `long_chat_result_generated_context_bytes`
  and `long_chat_result_generated_context_hash` when that identity is present.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate state_capsule_wrapped_assistant_context_preserves_generated_identity_summary -- --nocapture
assertion failed: summary.contains("long_chat_summary_matching_generated_context_identity_links=3")
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate state_capsule_wrapped_assistant_context_preserves_generated_identity_summary -- --nocapture
test state_capsule_wrapped_assistant_context_preserves_generated_identity_summary ... ok

cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 66 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Limits

This is harness correctness for generated-context identity. It does not change
the request payload, model behavior, cache policy, or OpenAI server runtime.

The next real-model proof should rerun the queue gate with an assistant-context
state capsule and confirm `long_chat_summary_run_complete=true` on the rebuilt
binary.
