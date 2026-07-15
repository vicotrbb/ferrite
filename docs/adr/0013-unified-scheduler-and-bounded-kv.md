# ADR 0013: Unified scheduling and bounded KV memory

Date: 2026-07-13

Status: Accepted

## Context

Ferrite previously had two correct but separate serving paths. Prefix-cache and
trace requests used ordinary generation, while continuous batching accepted
only uncached streaming requests. The split prevented cached chat turns from
sharing weight reads with other requests. It also left server KV capacity on
the unbounded vector backend even though the CLI already exposed the optional
Locus block pool.

Correct unification must preserve fused greedy argmax, exact output identity,
per-request stop and usage semantics, cache namespace isolation, bounded queues,
and prompt cancellation. It must not retain full logits merely to make a cache
entry usable by the scheduler.

## Decision

The continuous scheduler owns every eligible fused-greedy chat or completion
request, whether the response is streaming or non-streaming and whether prefix
reuse or trace reporting is enabled.

For each request, the scheduler:

1. Tokenizes the complete rendered prompt and builds the normal compatibility
   key.
2. Leases the longest compatible immutable prefix snapshot.
3. Restores that state into an independent mutable session.
4. Batches only uncached prompt positions, then advances every ready decode
   stream by one token per scheduler cycle.
5. Stores the complete prompt snapshot with only its greedy next-token ID.

The last rule preserves fused argmax and avoids retaining a full logit vector.
If a later sampled request finds such an exact entry, ordinary generation
restores all but the final prompt token and evaluates that token once to recover
full logits. Cached-token accounting reports only positions actually reused.

Equal-prompt prefill sharing requires equality of the complete cache options,
including namespace. Prefix snapshots use `Arc` ownership. Eviction releases
the cache's owner, while any in-flight restore lease remains valid. Entry-count
and logical-byte budgets are configurable and enforced together.

Batch active-stream count, waiting queue length, and per-stream event channels
are bounded separately. Closed receivers are discarded before admission,
during prompt prefill, or at the next token boundary. A backpressured response
pauses only its own decode eligibility.

The server also exposes the optional Locus KV backend behind the existing
`locus-kv` Cargo feature. Locus uses fixed-size mapped blocks with stale-handle
validation and an explicit per-session token cap. Requests that cannot fit
their prompt plus worst-case decode state fail before prompt evaluation. Prefix
snapshot memory remains a separate configured budget.

## Consequences

Cached and uncached greedy requests now share one scheduling contract and one
response-semantic path. Streaming no longer determines scheduler eligibility.
Sampled or logit-modified requests remain outside continuous batching because
they require full logits.

Immutable cache leases remove deep snapshot clones during lookup and make
eviction safety explicit. Restoring a lease still copies KV into a request's
independent mutable session. The current Locus pool is per session, so its
configured cap must be multiplied by the maximum admitted session count when
sizing a process.

The default vector backend and all experimental opt-ins remain compatible.
No performance claim follows from this architecture change until clean,
comparable eval artifacts pass the documented gates.

## Alternatives Considered

- Keep prefix requests on ordinary generation. Rejected because it preserves
  the architectural split and forfeits batchable uncached suffix work.
- Store complete logits in scheduler cache entries. Rejected because it widens
  memory use and violates the fused-greedy fast-path contract.
- Share mutable sessions between requests. Rejected because cancellation,
  stop handling, and ownership become race-prone.
- Fall back to vector allocation when Locus is exhausted. Rejected because it
  makes the configured memory ceiling dishonest.

## Evidence

- `crates/ferrite-server/src/runtime/scheduler.rs` implements prefix restore,
  suffix-only prefill, namespace-safe deduplication, fairness, and cancellation.
- `crates/ferrite-server/src/runtime/prefix_cache.rs` implements immutable
  leases, budgets, safe eviction, and greedy cache entries.
- `crates/ferrite-server/src/openai/generation.rs` collects scheduler events for
  non-streaming responses.
- `crates/ferrite-server/src/config.rs` validates queue, snapshot, and Locus
  limits.
- Runtime and route tests cover scheduler cache reuse, sampled-logit recovery,
  namespace isolation, eviction leases, churn, and streaming plus non-streaming
  usage accounting.
- `scripts/eval.py` and `scripts/eval_suite.py` provide the real-model token,
  TTFT, throughput, and RSS gates. No new performance result is recorded here.
