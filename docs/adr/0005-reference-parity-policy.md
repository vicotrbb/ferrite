# ADR 0005: Model Output Reference Parity Policy

Date: 2026-06-27

Status: Accepted

## Context

Ferrite's Tier 0 gate requires deterministic model output that can be compared
against a documented `llama.cpp` reference. The SmolLM2-135M probe is stable
across checked local `llama.cpp` modes: default, CPU-only, and CPU no-repack
all produce the same six-token continuation for `hello world`.

The SmolLM2-360M probe exposed a reference ambiguity. Ferrite matches the
default local `llama.cpp` greedy continuation, but CPU-only `llama.cpp` and
CPU-only no-repack `llama.cpp` produce different continuations after the first
generated token. Ferrite's top-logits diagnostic shows the first divergent
candidates are close: token `284` at `18.689020` and token `198` at
`18.645466`.

That means "matches `llama.cpp`" is underspecified for quantized near-ties.
Different `llama.cpp` backends and repacking modes can be valid comparison
runtimes while still choosing different argmax tokens after accumulated
quantized-kernel rounding.

## Decision

Ferrite model-output parity claims must name a complete reference profile:

- reference runtime and commit,
- model artifact and file hash or Hugging Face revision when available,
- prompt text and prompt token IDs,
- generation length,
- sampling mode,
- backend/device mode,
- relevant backend toggles such as repacking or flash attention,
- exact command output or tokenization evidence.

Exact token parity is valid only relative to that declared profile. Ferrite must
not claim parity with all `llama.cpp` CPU or accelerator backends unless those
backends have actually been checked and agree.

For Tier 0, exact token parity against a fixed, documented `llama.cpp` greedy
profile is sufficient to prove the model-plumbing gate when paired with:

- parser-load evidence,
- scalar forward output evidence,
- streaming-mode evidence,
- memory and latency notes,
- documentation of any known reference-backend disagreement.

Backend-sensitive near-ties must be documented as a bounded caveat, not hidden.
They do not by themselves prove Ferrite is wrong, and they also do not certify
Ferrite's quantized arithmetic as numerically equivalent to every CPU backend.

CPU-only parity remains a separate validation target. Before optimized SIMD,
threaded, mmap-backed, or architecture-specific CPU kernels are treated as
correct, Ferrite must compare those optimized paths against the Ferrite scalar
reference path and record CPU-backend comparison evidence.

## Consequences

Tier gates become reproducible because every output claim is tied to a concrete
reference profile instead of an implicit `llama.cpp` default.

Ferrite can make progress through the Tier 0 plumbing gate despite an external
backend disagreement, provided the disagreement is documented and bounded.

This policy does not weaken future optimized-kernel requirements. Tier 1 and
later SIMD work still needs numerical comparison against Ferrite's scalar
reference path and benchmark evidence on CPU targets.

This policy also means some output comparisons may be classified as "partial"
instead of "failed" when external reference backends disagree and the observed
candidate margins are close.

## Alternatives Considered

Require parity with every `llama.cpp` backend.

This was rejected because local evidence shows `llama.cpp` default, CPU-only,
and CPU no-repack modes can disagree on a quantized near-tie for the same model
and prompt. Treating all disagreement as Ferrite failure would make the gate
depend on an unstable external target.

Require CPU-only `llama.cpp` parity for Tier 0.

This was rejected as the Tier 0 model-plumbing gate because Tier 0's purpose is
to prove parser, scalar forward, deterministic output, and streaming plumbing.
CPU-only backend parity is important, but it is a separate numerical/backend
validation question when `llama.cpp` CPU modes disagree with each other.

Ignore backend disagreement.

This was rejected because Ferrite's correctness claims must remain evidence
scoped. Known reference splits must be recorded so later kernel work does not
mistake a backend-sensitive near-tie for broad correctness.

## Evidence

- `crates/ferrite-cli/tests/` covers expected next-token and generated-token
  traces as executable CLI gates.
- `crates/ferrite-inference/tests/scalar_reference.rs` keeps the scalar path as
  the internal correctness reference.
- [`../evaluation.md`](../evaluation.md) requires model identity, commands,
  fixed inputs, and token traces for parity claims.
- [`../benchmarks/2026-07-10-oss-quality-hardening.md`](../benchmarks/2026-07-10-oss-quality-hardening.md)
  records exact 512-token parity for the current optimized policy.
