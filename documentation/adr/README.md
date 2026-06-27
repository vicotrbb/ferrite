# Architecture Decision Records

ADRs capture durable technical decisions. Use one ADR per decision.

Create files as:

`NNNN-short-title.md`

Example:

`0001-documentation-and-iteration-model.md`

## Required When

Write or update an ADR when Ferrite changes:

- Runtime architecture.
- Model format support.
- Dependency policy.
- Unsafe-code boundary.
- Kernel strategy.
- KV cache layout.
- Memory allocation or streaming strategy.
- Benchmark gate.
- Homelab or deployment assumptions.

## Template

```markdown
# ADR NNNN: Title

Date: YYYY-MM-DD

Status: Proposed | Accepted | Superseded | Rejected

## Context

What forces, evidence, constraints, or prior work led to this decision?

## Decision

What are we deciding?

## Consequences

What becomes easier, harder, riskier, or explicitly out of scope?

## Alternatives Considered

Which credible alternatives were rejected, and why?

## Evidence

What repo files, commands, benchmarks, papers, or experiments support this?
```
