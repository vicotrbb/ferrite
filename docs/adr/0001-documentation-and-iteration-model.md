# ADR 0001: Documentation and iteration model

Date: 2026-06-27

Status: Superseded by [ADR 0012](0012-open-source-quality-baseline.md)

## Context

CPU inference work is sensitive to model identity, hardware, numerical order,
memory layout, and benchmark method. Ferrite needed an evidence trail that kept
hypotheses, durable decisions, implementation, and measured results distinct.

## Decision

The original project model required a development note for each work slice,
benchmark evidence for measured claims, research notes for new hypotheses, and
an ADR for durable choices. Those artifacts were split across `research/` and
`documentation/` directory trees.

## Consequences

The evidence-first loop prevented speculative kernel work from being presented
as an optimization and established the benchmark and parity discipline that
Ferrite still uses.

The per-session artifact rule also produced hundreds of short files that
duplicated Git history and obscured the maintained contract. ADR 0012 now keeps
only durable ADRs, current technical guidance, curated milestone benchmarks,
and machine-readable eval records in the public repository.

## Alternatives considered

- **Code without evidence.** Rejected because hot-path and numerical changes
  cannot be reviewed reliably from code shape alone.
- **Retain every process artifact forever.** Superseded because Git already
  preserves history, while a public repository needs a concise current path.

## Evidence

- [`../evaluation.md`](../evaluation.md) defines the current regression gates.
- [`../benchmarks/README.md`](../benchmarks/README.md) defines retained
  milestone evidence.
- [`../../scripts/evals/`](../../scripts/evals/) contains raw eval records.
