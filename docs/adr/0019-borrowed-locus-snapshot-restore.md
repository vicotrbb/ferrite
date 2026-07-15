# ADR 0019: Borrowed Locus snapshot restore

Date: 2026-07-14

Status: Accepted

## Context

The continuous scheduler evaluates one representative for a group of identical
prompts, creates one backend-independent KV snapshot, and restores it into each
duplicate session. The vector backend must own cloned vectors after restore.
The Locus backend instead writes KV values into its own mapped block pool, but
its restore path first deep-cloned the complete snapshot before copying those
temporary vectors into the mapping.

Phi-3 Mini has a 3,072-value KV row per layer. For a short prompt, restoring
three duplicate sessions can therefore create tens of MiB of avoidable private
heap churn. Two retained acceptance attempts exposed the result as continuous-
batched physical-footprint instability above the fixed 16 MiB soak limit. One
attempt moved from 48,973,936 to 14,289,008 bytes; another moved from 19,368,048
to 46,024,816 bytes. Exact token traces remained stable, which isolated the
problem to restore allocation rather than model state.

## Decision

Backend-independent snapshots expose crate-private borrowed layer views in
addition to the existing owned-clone accessors. Locus restore iterates those
borrowed key and value rows and copies each row directly into its mapped block.
Its normal owned `push` API delegates to the same slice-based implementation.

The vector backend keeps the owned-clone restore path because the cloned
vectors become its session storage. Snapshot structure, public API behavior,
KV values, capacity checks, and deterministic token IDs do not change.

## Consequences

Restoring an identical prompt into a Locus session no longer creates a second
complete heap copy of the snapshot. The unavoidable mapped-session copy remains
independent and mutable, preserving cancellation and ownership isolation.

This change targets allocation stability. It does not imply a throughput or
TTFT improvement. Clean repeated artifacts remain required for performance
claims, and the 16 MiB growth and tail-range gates remain unchanged.

The crate-private snapshot surface grows by two borrowed accessors. They do not
expose mutable state or widen the public compatibility contract.

## Alternatives Considered

- Increase the soak tolerance. Rejected because it would weaken a gate that
  already found a real whole-matrix row-decoding defect.
- Treat large downward footprint changes as automatically safe. Rejected
  because symmetric tail range previously exposed allocator churn caused by
  request-sized temporary allocations.
- Share one mutable Locus pool between requests. Rejected for this change
  because it would require a larger scheduler ownership redesign.
- Disable identical-prompt prefill sharing. Rejected because it removes useful
  weight-sharing work instead of fixing the restore allocation.

## Evidence

- `LocusKvStore::restore` now consumes borrowed snapshot rows through
  `push_slices`; `locus_store_snapshot_round_trip` covers exact restoration
  across mapped block boundaries.
- The rejected repetition-2 artifacts are
  [`2026-07-14-162517`](../../scripts/evals/2026-07-14-162517-qwen2.5-1.5b-instruct-q8_0-multi.md)
  and
  [`2026-07-14-171719`](../../scripts/evals/2026-07-14-171719-qwen2.5-1.5b-instruct-q8_0-multi.md).
- The focused post-change
  [`2026-07-14-175626`](../../scripts/evals/2026-07-14-175626-phi-3-mini-4k-instruct-q4.md)
  Phi-3 diagnostic preserved exact default and batched traces. Continuous-
  batched physical-footprint growth was 409,600 bytes and its tail range was
  540,672 bytes, both below the unchanged 16 MiB limit. This diagnostic is not
  clean-host performance evidence.
- The final clean repeated suite must still pass before this decision supports
  a performance claim.
