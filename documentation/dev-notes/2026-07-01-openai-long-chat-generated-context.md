# OpenAI Long-Chat Generated Context Carry-Forward

## Context

The Tier 1 OpenAI long-chat gate requires repeated multi-turn conversations.
Previous harness runs labeled scenarios by turn, but each scenario reused the
configured seed assistant context instead of carrying generated assistant text
from the previous turn.

## Change

`ferrite-openai-long-chat-gate` now captures assistant-visible streaming text
from each completed scenario and uses it as the assistant context for the next
turn with the same `(model, token_length)` pair.

The first turn for each pair uses the configured seed assistant context. Every
later turn must use generated context from the previous completed scenario for
that same model and token-length sequence.

The harness now emits:

- `long_chat_result_assistant_context_source=seed|generated`;
- `long_chat_summary_all_follow_up_turns_use_generated_context`.

`long_chat_summary_run_complete=true` now requires every follow-up turn to use
generated assistant context.

## Validation

```sh
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
```

Result:

```text
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Remaining Scope

The OpenAI-compatible HTTP surface still reports `cached_tokens: 0`; it does
not expose internal KV-cache bytes for server requests. Future cache-specific
proof should either add a deliberate diagnostic surface or keep using the
existing inference-level KV-cache probes rather than inferring KV behavior from
OpenAI usage fields.
