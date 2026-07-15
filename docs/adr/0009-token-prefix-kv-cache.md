# ADR 0009: Token-prefix KV cache

Date: 2026-07-02

Status: Accepted as an experimental feature; scheduler restriction superseded by ADR 0013

## Context

Repeated chat turns re-evaluate large unchanged prompt prefixes. Ferrite
sessions already own per-layer key and value state, snapshots, truncation, and
byte accounting, so exact token-prefix reuse can reduce repeated prefill work.
Incorrect reuse would corrupt generation and unbounded retention would turn a
latency feature into a memory leak.

## Decision

Ferrite implements an opt-in, bounded token-prefix cache owned by the inference
and runtime layers. HTTP request metadata can namespace a lookup, but protocol
types are not cache identities.

A compatible key includes:

- model, tokenizer, and prompt-template fingerprints;
- exact ordered token identity and token count;
- execution policy and request-shape fingerprints;
- an optional non-empty caller namespace from `prompt_cache_key`.

The store validates token equality and selects the longest compatible shared
prefix. Unknown or mismatched state fails closed to full prompt evaluation.
Entries carry byte and use metadata and are evicted by explicit entry-count and
byte budgets.

Usage accounting remains truthful: `prompt_tokens` is the full prompt size and
`prompt_tokens_details.cached_tokens` is only the number of prompt tokens whose
KV state was actually reused. Streaming and non-streaming responses use the
same accounting.

The server flag `--experimental-prefix-cache` is required. Without it,
`prompt_cache_key` remains accepted compatibility metadata and enables no
reuse. Cache-enabled requests do not enter the continuous-batch scheduler.

## Consequences

Cache hits can reduce repeated-turn time to first token but do not improve
steady-state decode throughput. Memory use grows with retained snapshots, so
cache changes require byte-budget, eviction, isolation, cancellation, and RSS
tests.

The feature remains off by default while model coverage and long-context
evidence expand.

ADR 0013 later unified cache-enabled greedy requests with continuous batching.

## Alternatives considered

- **Trust only the caller cache key.** Rejected because a namespace is not
  proof that token prefixes are identical.
- **Place snapshots in OpenAI schemas.** Rejected because KV state belongs to
  inference, not a transport protocol.
- **Unbounded reuse.** Rejected because predictable memory is part of the
  server contract.

## Evidence

- `crates/ferrite-inference/src/prefix_cache.rs` defines token identities and
  fingerprints.
- `crates/ferrite-inference/src/prefix_cache/store.rs` implements bounded
  longest-compatible-prefix lookup and eviction.
- `crates/ferrite-inference/tests/token_prefix_cache.rs` covers identity,
  isolation, lookup, and budget behavior.
- `crates/ferrite-server/src/runtime/prefix_cache.rs` maps server requests to
  inference-owned cache state.
