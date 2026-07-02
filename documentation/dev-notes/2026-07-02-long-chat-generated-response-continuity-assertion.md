# Long-Chat Generated Response Continuity Assertion

## Context

The generated-context windowing theory now has strong prompt-length and TTFT
evidence for 32-token and 64-token windows, including reconnect/error probes at
the 512-token budget. The next proof gap is conversation continuity: the gate
needs a machine-checkable way to assert that generated follow-up responses still
carry a required marker or anchor phrase.

## Change

The long-chat gate now accepts repeated
`--require-generated-response-contains TEXT` flags.

The assertion applies only to turns whose assistant context source is
`generated`. Seed turns are intentionally excluded so a proof can focus on
follow-up continuity after Ferrite has carried prior generated assistant text
forward.

The gate fails if a generated follow-up response has no streaming text or if
the streaming text does not contain every required substring.

## Validation

Targeted red/green workflow:

```text
cargo test -p ferrite-server --test long_chat_gate generated_response -- --nocapture
```

The red run failed because `LongChatGateConfig` had no
`required_generated_response_substrings` method. After the implementation, the
full long-chat gate target passed:

```text
cargo test -p ferrite-server --test long_chat_gate -- --nocapture
41 passed; 0 failed
```

## Next Proof

Use this assertion in a real-model continuity run with the 32-token and
64-token generated-context windows. The assertion should remain a proof-gate
feature only; it does not change default OpenAI-compatible server behavior.
