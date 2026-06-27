# ADR 0001: Documentation and Iteration Model

Date: 2026-06-27

Status: Accepted

## Context

Ferrite is intended to be both a Rust CPU-native LLM inference engine and a
research vehicle for CPU inference improvements. The baseline research in
`research/` is useful, but it is not an implementation contract. The project
needs a repeatable way to turn research into code, evidence, decisions, and new
research without losing traceability.

The user explicitly requires ADRs, development notes, research documentation,
theory documentation, progressive model validation, rigorous Rust engineering
standards, and safe handling of local versus homelab execution.

## Decision

Ferrite will use `documentation/` as the authoritative project record for
engineering process and ongoing work:

- `documentation/engineering/` for project operating model and standards.
- `documentation/adr/` for durable decisions.
- `documentation/dev-notes/` for implementation and experiment logs.
- `documentation/research/` for research updates.
- `documentation/theories/` for speculative ideas and validation plans.
- `documentation/benchmarks/` for benchmark protocols and results.

Every meaningful iteration must produce development notes, validation evidence,
and benchmark notes when relevant. Durable technical choices must be recorded as
ADRs. Speculative ideas remain theory notes until evidence supports promotion.

## Consequences

Ferrite development will be slower than a code-only prototype at first, but the
project gains a durable evidence trail. This is necessary because CPU inference
work is sensitive to hardware, memory layout, numerical tolerances, and
benchmark methodology.

The baseline research can guide implementation, but new evidence can override
it through ADRs. The project must distinguish hypotheses, decisions, and proven
behavior.

## Alternatives Considered

Use only the existing `research/` directory.

This was rejected because the research corpus mixes survey material,
implementation ideas, timelines, and speculative theories. It does not provide
an operational record of what Ferrite has actually decided or proven.

Start coding immediately and document later.

This was rejected because undocumented architecture drift is especially costly
for low-level inference code. Ferrite needs traceability before it starts
building unsafe boundaries, parsers, kernels, and benchmark claims.

## Evidence

- `README.md` defines Ferrite as a CPU-native LLM inference engine.
- `research/README.md` provides the baseline target and research corpus.
- `research/11-testing-model-registry.md` defines progressive model tiers.
- `documentation/engineering/ferrite-operating-model.md` defines the project
  loop adopted by this ADR.
