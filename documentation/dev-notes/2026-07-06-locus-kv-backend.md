# Locus KV Backend

Date: 2026-07-06

## Goal

Give Ferrite's scalar-path KV cache a pooled, block-allocated storage backend
(Locus's `KvBlockPool`) as an opt-in alternative to the existing nested-`Vec`
storage, with zero correctness regression and no decode-throughput loss, per
`docs/superpowers/specs/2026-07-06-ferrite-locus-kv-cache-design.md`. This
note records the evidence-gathering slice: what is proven now on this host
(no Tier 1 model artifact available), and what remains pending a real-model
run.

## Context

- Design: `docs/superpowers/specs/2026-07-06-ferrite-locus-kv-cache-design.md`.
  Success gate: bit-identical logits/KV vs. the Vec backend, no decode-tok/s
  loss, and an improvement in at least one of {allocations per token,
  peak/post-load RSS}.
- The old KV storage was inline `Vec<Vec<Vec<f32>>>` fields directly on
  `ScalarLlamaSession`, with no seam for an alternate backend. This slice
  introduced `KvCacheStore` (`crates/ferrite-inference/src/scalar/kv_store.rs`)
  as that seam.
- No Tier 1 GGUF (e.g. Qwen2.5-1.5B) exists on this host: `target/models/` is
  absent and no `.gguf` file is tracked in the repo. Per Ferrite's operating
  model, this task did not download one. All real-model figures in this note
  are therefore explicitly marked PENDING with exact repro commands, not
  estimated.
- Full evidence and repro commands: `documentation/benchmarks/2026-07-06-locus-kv-backend.md`.

## Changes

- `KvCacheStore` enum (`Vec` | `Locus`, the latter `#[cfg(feature =
  "locus-kv")]`) behind a stable interface (`push`, `key`, `value`,
  `truncate`, `kv_cache_bytes`, `snapshot`, `restore`, `layer_count`,
  `layer_len`). `Vec` reproduces the historical nested-`Vec` behavior exactly
  and remains the default; nothing changes for callers that don't opt in.
- `LocusKvStore` (`crates/ferrite-inference/src/scalar/kv_store/locus.rs`):
  wraps a `locus_alloc::KvBlockPool` (mapped, LIFO reuse). One block list per
  (layer, K/V); each block holds `tokens_per_block` positions. A block is
  allocated lazily on first write to a new block boundary
  (`ensure_block`/`write_block`); `truncate` frees blocks back to the pool
  down to the number needed for the retained token count; `kv_cache_bytes`
  reports the same logical f32-byte accounting as the Vec backend (position
  count × head_kv_dim × 4 bytes × 2, not pool capacity) so the two backends
  are comparable apples-to-apples.
- Allocation is opt-in at two levels: the `locus-kv` cargo feature (added to
  `ferrite-inference` and threaded through `ferrite-cli`'s own `locus-kv`
  feature) must be compiled in, and the caller must explicitly request
  `KvBackend::Locus { tokens_per_block, max_tokens }` (CLI:
  `--kv-backend locus --kv-tokens-per-block <n> --kv-max-tokens <n>`, default
  `tokens_per_block = 16`). Without both, behavior and code path are
  byte-for-byte the pre-existing Vec path.
- The CLI prints `locus_pool_allocation_count` (from
  `pool_stats().allocation_count`, Locus's cumulative successful-allocation
  counter — distinct from `pool_stats().allocated`, the current live-block
  count) when the Locus backend is active, purely for this kind of
  allocation-mechanics evidence gathering.
- `ferrite-server` has no `locus-kv` feature and no `--kv-backend` flag; the
  server-side opt-in path does not exist yet (confirmed by grep — see the
  benchmark note's "Server opt-in path" section). This is intentional scope
  boundary for this slice, not an oversight.

## Validation

1. Differential correctness parity, real forward path:

   ```sh
   cargo test -p ferrite-inference --features locus-kv --test kv_store_backend_parity -- --nocapture
   ```

   ```text
   test locus_backend_matches_vec_logits ... ok
   test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   ```

   Runs the same 5-token prompt (spanning a block boundary) through the Vec
   and Locus backends against a fixture GGUF whose Q/K/V/O and FFN weights
   genuinely mix across positions, and asserts bit-identical `token_id`,
   `logits`, and `kv_cache_bytes`.

2. `LocusKvStore` unit tests (allocation mechanics):

   ```sh
   cargo test -p ferrite-inference --features locus-kv --lib scalar::kv_store::locus -- --nocapture
   ```

   ```text
   test scalar::kv_store::locus::tests::locus_store_round_trips_across_block_boundaries ... ok
   test scalar::kv_store::locus::tests::locus_store_reports_out_of_blocks ... ok
   test scalar::kv_store::locus::tests::locus_store_truncates_and_frees_blocks ... ok
   test scalar::kv_store::locus::tests::locus_store_snapshot_round_trip ... ok
   test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.00s
   ```

   Covers round-tripping across multiple blocks, block reclamation on
   truncate (`pool_stats().allocated` strictly decreases), and an explicit
   out-of-blocks error instead of silent corruption.

3. Release build with the feature:

   ```sh
   cargo build --release -p ferrite-cli --features locus-kv
   ```

   Succeeds (`Finished \`release\` profile [optimized] target(s)`).

4. End-to-end CLI check on a synthetic single-layer fixture GGUF (optional
   step in the evidence task, achieved via a throwaway test that wrote the
   fixture bytes to a scratchpad path and was deleted immediately — no test
   churn committed): both `--kv-backend vec` (default) and `--kv-backend
   locus --kv-tokens-per-block 2` produced identical `next_token_id`,
   `generated_token_ids`, and `kv_cache_bytes` through the CLI. The Locus run
   additionally reported `locus_pool_allocation_count=8`, matching the
   analytic formula `2 * num_layers * ceil(tokens/tokens_per_block) = 2 * 1 *
   ceil(8/2) = 8` for that fixture exactly. This proves CLI + Locus plumbing
   end to end; the model is a few-KB synthetic fixture, so its timing/RSS are
   not meaningful and are not reported.

Full commands and pasted output: `documentation/benchmarks/2026-07-06-locus-kv-backend.md`.

## Results

**Proven:**

- Correctness: bit-identical logits, token IDs, and `kv_cache_bytes` between
  Vec and Locus backends, both at the library level (differential test) and
  end to end through the CLI (fixture GGUF).
- Allocation mechanics: block allocation, reclamation, and reuse work as
  designed (unit tests); the allocation-count formula
  `2 * num_layers * ceil(tokens/tokens_per_block)` matches a real measured
  counter on the CLI fixture run.
- The feature is fully opt-in (feature flag + explicit CLI flag); the default
  Vec path is untouched.
- Release build with the feature compiles cleanly.

**Pending (needs a real Tier 1 model artifact + explicit download
authorization, not measured in this slice):**

- Real-model correctness parity (same token id, Vec vs. Locus, on dequantized
  weights rather than the tiny fixture).
- Real-model allocation churn count over a longer generation.
- Peak and post-load RSS, both backends, short and long sequence lengths —
  the design's actual success-gate metric.
- Decode throughput (tok/s) via `--benchmark-runs`, both backends — the
  design's no-regression gate.
- x86_64 build/behavior (this host is Apple Silicon only).
- NUMA behavior — explicitly out of scope per the design spec (Locus's `numa`
  feature is Linux-only and not enabled here).
- The server-side opt-in path does not exist yet (no `locus-kv` feature or
  `--kv-backend` flag in `ferrite-server`) — not "unmeasured," genuinely
  unimplemented.

Every PENDING item above has an exact repro command in the benchmark note so
a future session with a model artifact in hand can fill in the numbers
without re-deriving the protocol.

## Follow-Ups

- Get explicit authorization to fetch a Tier 1 artifact (or use one already
  present on a suitable host) and run the PENDING commands verbatim to close
  out the design's success gate with real RSS/throughput numbers.
- If RSS/throughput prove out, consider wiring a `--kv-backend`/`locus-kv`
  opt-in path into `ferrite-server` (currently absent) so the server surface
  can benefit too.
- Longer-context runs (beyond the 32/512-token samples suggested in the
  benchmark note) to see whether the Locus backend's RSS advantage (if any)
  grows with sequence length, per the design's "ideally lower at longer
  lengths" expectation.
- x86_64 verification once a suitable host is available.
