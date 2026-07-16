# Architecture Decision Records

ADRs capture durable technical decisions. Use one ADR per decision.

## Accepted decisions

1. [Documentation and iteration model](0001-documentation-and-iteration-model.md)
2. [Rust workspace and GGUF reader](0002-rust-workspace-and-gguf-reader.md)
3. [Scalar reference inference boundary](0003-scalar-reference-inference-boundary.md)
4. [Tokenizer boundary](0004-tokenizer-boundary.md)
5. [Reference parity policy](0005-reference-parity-policy.md)
6. [SIMD unsafe boundary](0006-simd-unsafe-boundary.md)
7. [Q8_K activation dot path](0007-q8-k-activation-dot-path.md)
8. [OpenAI-compatible HTTP API](0008-openai-compatible-http-api.md)
9. [Token-prefix KV cache](0009-token-prefix-kv-cache.md)
10. [Locus KV block backend](0010-locus-kv-block-backend.md)
11. [Concurrent serving and batched decode](0011-concurrent-serving-and-batched-decode.md)
12. [Open source quality baseline](0012-open-source-quality-baseline.md)
13. [Unified scheduler and bounded KV](0013-unified-scheduler-and-bounded-kv.md)
14. [Portable kernel provider and thread topology](0014-portable-kernel-provider-and-thread-topology.md)
15. [Architecture normalization and verified Phi-3](0015-architecture-normalization-and-verified-phi3.md)
16. [Bounded structured output, tools, and Responses](0016-bounded-structured-output-tools-and-responses.md)
17. [Defer speculative decoding until clean gates](0017-defer-speculative-decoding-until-clean-gates.md)
18. [Retained matrix storage and bounded row decoding](0018-retained-matrix-storage-and-bounded-row-decoding.md)
19. [Borrowed Locus snapshot restore](0019-borrowed-locus-snapshot-restore.md)
20. [Rust 2024 and bounded test harnesses](0020-rust-2024-and-bounded-test-harnesses.md)

Create files as:

`NNNN-short-title.md`

Example:

`0001-documentation-and-iteration-model.md`

## Required When

Write or update an ADR when Ferrite changes:

- Runtime architecture.
- Model format support.
- Dependency policy.
- HTTP API compatibility surface.
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
