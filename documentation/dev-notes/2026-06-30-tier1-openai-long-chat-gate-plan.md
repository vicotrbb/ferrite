# Tier 1 OpenAI long-chat gate plan

## Context

The next OpenAI-compatible proof milestone needs to move beyond one-token and
32-token streaming chat evidence. The requested gate covers 256, 512, and
1024-token streaming responses, repeated multi-turn conversations, RSS
sampling, per-token latency, stop/EOS behavior, and reconnect/error behavior.

## Slice

Add a dedicated engineering gate specification:

- `documentation/engineering/tier1-openai-long-chat-gate.md`

Update Tier 1 status so long-chat readiness cannot be claimed from the narrower
existing streaming evidence.

## Validation

This is a documentation/planning slice. It defines pass criteria and required
artifacts; it does not run long-chat probes yet.

Follow-up implementation should add a focused long-chat measurement harness or
extend the existing OpenAI throughput client with SSE token timing and RSS
sampling before any benchmark note claims this gate.
