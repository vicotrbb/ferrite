# ADR 0017: Defer speculative decoding until clean gates

Date: 2026-07-13

Status: Accepted

## Context

Speculative decoding can improve some decode workloads, but it adds another
model or candidate source, acceptance logic, sampling interactions, failure
fallbacks, and retained memory. It can also worsen TTFT or realistic throughput
when candidate acceptance is low.

Ferrite now has deterministic sampling, unified greedy scheduling, bounded KV
storage, portable dispatch, and a useful 3.8B correctness proof. The milestone
host nevertheless had material unrelated background load. Clean repeated
throughput, TTFT, steady-state RSS, and 3.8B performance artifacts were not
available. No draft model with pinned provenance was selected.

## Decision

Do not implement or ship speculative decoding in this milestone. Entry into a
speculative experiment requires all of the following on one comparable setup:

1. clean repeated baseline `scripts/eval.sh` artifacts;
2. exact deterministic token parity for the requested sampling policy;
3. pinned target and draft model provenance, or a deterministic prompt or
   n-gram candidate policy;
4. acceptance-rate, TTFT, sustained-throughput, peak-RSS, and idle-RSS records;
5. explicit fallback behavior for rejection, cancellation, context limits,
   draft failure, and unsupported sampling;
6. realistic single-stream and concurrent prompt workloads;
7. retention only when median end-to-end results improve without changing
   requested semantics.

The first experiment should be bounded and removable. It must not share mutable
sampler or KV state between requests, and it must compare against the same model
bytes, prompt set, output budgets, build, thread policy, and host state.

## Consequences

Ferrite keeps one correctness path while the prerequisite evidence is
incomplete. No draft-model memory, dependency, API, or fallback complexity is
added speculatively.

This decision is not evidence that speculative decoding is slow. It records
that the experiment is currently ineligible, preventing contaminated results
or incomplete prerequisites from becoming a product claim.

## Alternatives Considered

- Add a draft model immediately and benchmark later. Rejected because the
  memory and sampling contracts would land before an acceptance gate.
- Add n-gram speculation because it needs no second model. Rejected for the
  same evidence reason; simpler provenance does not remove acceptance, TTFT,
  fallback, or workload requirements.
- Report an opportunistic result from the busy host. Rejected because Ferrite's
  evaluation policy excludes material background load.

## Evidence

- `docs/acceptance-matrix.md` records the clean-run, parity, TTFT, throughput,
  and RSS prerequisites.
- `scripts/eval.sh`, `scripts/eval_suite.py`, and
  `scripts/reference_compare.py` provide the reproducible comparison surface.
- ADR 0013 records unified scheduling and bounded KV requirements.
- ADR 0014 records portable dispatch and the no-performance-claim boundary.
- No speculative code or performance artifact is retained by this decision.
