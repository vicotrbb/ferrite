# ADR 0009: Token-Prefix KV Cache

Date: 2026-07-02

Status: Proposed

## Context

The Tier 1 generated-context long-chat proofs show that repeated OpenAI-style
chat turns spend a large amount of time reprocessing prompt tokens that are
unchanged from the previous request. The Qwen2.5-1.5B Q8_0 x86_64 timing
probes at 256, 512, and 1024 completion tokens show generated follow-up
first-token delay scaling much faster than the seed turn.

Ferrite already has the core primitive needed for a cautious cache design:
`ScalarLlamaSession` owns per-layer K/V state, tracks cached token count, can
report K/V cache bytes, and can truncate cached state. The OpenAI-compatible
chat schema already accepts `prompt_cache_key` as request metadata, but usage
currently reports `prompt_tokens_details.cached_tokens = 0` because no cross
request prompt-cache behavior exists.

This decision is required before implementation because KV cache layout,
retention, eviction, and OpenAI-compatible cached-token accounting affect both
the inference core and the server product surface.

## Decision

Ferrite will add an opt-in token-prefix KV cache as an inference-owned feature.
The server may pass request metadata and expose usage counters, but HTTP types
must not become part of the inference cache key or cached value.

The initial design has three phases:

1. Add token-prefix identity and observability without changing generation.
2. Add exact-prefix reuse behind an internal experiment flag.
3. Add bounded longest-prefix reuse only after exact-prefix reuse is proven.

The cache key must be token-level, not string-level. A usable key includes:

- loaded model identity and model-format fingerprint;
- tokenizer and chat-template fingerprint;
- exact token prefix hash plus prefix token count;
- execution policy that can affect logits, including scalar/SIMD policy;
- request-shape fields that affect rendered prompt tokens or termination;
- optional client-provided `prompt_cache_key` as an extra namespace, not as
  proof that two prompts are equivalent.

The cache value belongs in inference/session modules and records:

- the matched prefix token count;
- per-layer K/V state or a future compact equivalent;
- estimated K/V bytes;
- creation and last-used metadata for eviction;
- model and tokenizer fingerprints needed for fail-closed invalidation.

Cache lookup must fail closed. Ferrite must recompute the full prompt when any
fingerprint, token prefix, execution policy, model load, tokenizer, template,
or supported request-shape invariant is uncertain. Cache entries must be
evicted by explicit byte and entry-count budgets. Model reload must invalidate
all entries for that model instance.

OpenAI-compatible usage accounting must remain truthful. Until real reuse
occurs, `cached_tokens` remains `0`. Once reuse is enabled,
`prompt_tokens` remains the full prompt token count seen by the request, while
`prompt_tokens_details.cached_tokens` reports the number of prompt tokens whose
K/V state was reused. Streaming usage chunks must follow the same accounting.

The first implementation slice must stay small and modular:

- `ferrite-inference` gets token-prefix cache identity and cache-entry types;
- `ScalarLlamaSession` gets explicit snapshot or resume APIs only after tests
  prove equivalence with full-prompt recomputation;
- `ferrite-server` maps OpenAI request metadata into generic cache options;
- response usage gets cached-token plumbing only after the inference result can
  report real cached-token counts.

## Consequences

This design targets first-token latency for repeated local chat turns. It does
not claim to fix decode throughput, because the timing probes also show
post-first-token decode slowdown on generated-context turns.

The cache increases memory-retention risk. Every implementation phase must
track cache bytes directly and must rerun the long-chat RSS gates before any
cache mode can become a default. Aggressive eviction may reduce hit rate, but
predictable memory behavior is more important than optimistic caching.

The implementation must preserve Ferrite's modularity. Server code may parse
`prompt_cache_key`, but inference code owns token identity, session snapshots,
K/V cache layout, memory accounting, and reuse correctness.

The cache remains disabled by default until repeated 256, 512, and 1024-token
long-chat proofs show:

- lower generated follow-up first-token latency;
- unchanged response-shape and streaming behavior;
- unchanged stop and token-limit behavior;
- correct `cached_tokens` accounting;
- bounded RSS after idle and repeated sessions;
- passing request-error and client-disconnect reconnect probes.

## Alternatives Considered

Add generated-context windowing first.

This could reduce prompt size without K/V reuse, but it changes conversation
semantics and is better treated as a benchmark-only theory until users can
choose an explicit context policy.

Use the client-provided `prompt_cache_key` as the primary cache key.

This was rejected because it is metadata, not proof of token identity. It can
namespace cache entries, but Ferrite must still verify token-level prefix
equality before reusing K/V state.

Implement direct longest-prefix reuse immediately.

This was rejected as the first implementation step because partial-prefix reuse
has more correctness and eviction edge cases. Exact-prefix reuse is a safer
proof slice after identity and observability exist.

Put cache entries in the OpenAI server layer.

This was rejected because K/V state is inference-core state. Keeping it behind
server types would couple protocol compatibility to model execution internals.

## Evidence

- `documentation/theories/2026-07-02-long-chat-prefix-reuse.md` records the
  prefix-reuse hypothesis and the 256, 512, and 1024-token timing probes.
- `documentation/theories/2026-07-02-kv-cache-memory-pressure.md` records the
  memory-risk hypothesis and the need for explicit K/V byte accounting.
- `documentation/theories/2026-07-02-generated-context-windowing.md` records
  the alternative generated-context-window theory.
- `documentation/research/2026-07-02-llama-benchy-benchmark-companion.md`
  records `llama-benchy` as a possible OpenAI-compatible companion benchmark,
  not as a replacement for Ferrite's long-chat gate.
- `documentation/dev-notes/2026-06-29-openai-prompt-cache-key.md` records that
  `prompt_cache_key` is accepted today only as local metadata.
- `documentation/adr/0003-scalar-reference-inference-boundary.md` records the
  scalar session K/V cache boundary that future optimized paths must match.
- `documentation/adr/0008-openai-compatible-http-api.md` records the server and
  inference-core separation that this cache design preserves.
