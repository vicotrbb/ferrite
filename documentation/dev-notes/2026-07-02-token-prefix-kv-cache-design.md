# Token-Prefix KV Cache Design

Date: 2026-07-02

## Slice

This slice promotes the tested long-chat prefix-reuse theory into a proposed
architecture decision without changing production Rust code.

The goal is to make the next implementation step precise enough to avoid a
string-level or server-owned prompt cache. The design keeps K/V state inside
the inference boundary, treats OpenAI `prompt_cache_key` as metadata only, and
requires token-level prefix identity before reuse.

## Evidence Read

- `documentation/engineering/ferrite-operating-model.md`
- `documentation/engineering/ferrite-operating-model.md`
- `documentation/adr/README.md`
- `documentation/adr/0003-scalar-reference-inference-boundary.md`
- `documentation/adr/0008-openai-compatible-http-api.md`
- `documentation/theories/2026-07-02-long-chat-prefix-reuse.md`
- `documentation/theories/2026-07-02-kv-cache-memory-pressure.md`
- `documentation/theories/2026-07-02-generated-context-windowing.md`
- `documentation/research/2026-07-02-llama-benchy-benchmark-companion.md`
- `crates/ferrite-inference/src/scalar/session.rs`
- `crates/ferrite-inference/src/scalar/session/cache.rs`
- `crates/ferrite-server/src/openai/schema/prompt_cache_key.rs`
- `crates/ferrite-server/src/openai/schema/usage.rs`

## Result

Added `documentation/adr/0009-token-prefix-kv-cache.md` as a proposed ADR.

The ADR chooses an opt-in three-phase path:

1. token-prefix identity and observability with no behavior change;
2. exact-prefix K/V reuse behind an internal experiment flag;
3. bounded longest-prefix reuse only after exact-prefix proof.

The ADR also records the required cache-key inputs, cache-value ownership,
fail-closed invalidation rules, memory-budget expectations, and
OpenAI-compatible `cached_tokens` accounting semantics.

## Limits

No production cache code was added in this slice. The next implementation slice
still needs red tests for token-prefix identity before adding cache lookup or
session snapshot/resume behavior.

No performance claim is made from this ADR. The existing timing probes justify
the design work, but cache performance remains unproven until a later gated
implementation reruns the long-chat matrix.
