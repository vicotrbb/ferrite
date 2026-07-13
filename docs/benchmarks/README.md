# Curated benchmark evidence

This directory retains milestone-level methods, accepted results, and rejected
experiments that still inform Ferrite's current architecture.

- [`2026-07-06-locus-kv-backend.md`](2026-07-06-locus-kv-backend.md) records
  the optional block-pool KV backend contract and its proven scope.
- [`2026-07-09-concurrent-serving-phase1.md`](2026-07-09-concurrent-serving-phase1.md)
  records concurrent serving and batching evidence.
- [`2026-07-10-oss-quality-hardening.md`](2026-07-10-oss-quality-hardening.md)
  records the preceding full performance and parity gate.
- [`2026-07-13-memory-mapping-and-shared-prefill.md`](2026-07-13-memory-mapping-and-shared-prefill.md)
  records the current zero-copy model-storage and repeated 131.45 to 159.58
  tok/s shared-prompt server gate.

Machine-readable eval runs live under [`../../scripts/evals/`](../../scripts/evals/).
The eval artifacts are retained because they support comparable regression
checks. Session notes, contaminated runs, speculative theories, and superseded
micro-experiments belong in Git history, not this maintained index.

Every new benchmark note must identify the commit, toolchain, model hash, host,
build flags, prompt, token count, worker count, command, repeated-run statistic,
token-parity result, memory, CPU, and acceptance decision.
