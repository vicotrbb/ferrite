# Ferrite Documentation

Ferrite is built through an evidence-driven loop: research, decide, implement,
validate, benchmark, document, and repeat. This directory is the project record
for that loop.

## Structure

- `engineering/` - operating model, quality gates, engineering policies, and
  project-level standards.
- `adr/` - architecture decision records. Any durable technical direction,
  rejected alternative, unsafe boundary, dependency policy, or benchmark gate
  belongs here.
- `dev-notes/` - implementation logs for concrete work sessions and milestones.
- `research/` - focused research notes that refine, correct, or extend the
  baseline material in `../research`.
- `theories/` - speculative or novel ideas that need validation before they can
  become implementation work.
- `benchmarks/` - benchmark protocols, raw observations, summaries, and
  comparison notes.

## Required Trail

Every meaningful Ferrite iteration must leave these artifacts:

1. A development note describing the implementation or experiment.
2. Benchmark or validation evidence when behavior or performance is claimed.
3. An ADR when the work makes or changes a durable decision.
4. A research or theory note when the work explores a new technical direction.

Claims without evidence stay marked as hypotheses.
