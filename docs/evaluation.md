# Evaluation and regression gates

Ferrite separates fast deterministic tests from real-model and performance
gates. A code change is complete only after the checks proportional to its risk
have passed.

The [acceptance matrix](acceptance-matrix.md) maps model, workload, platform,
comparison, and measurement coverage. It also names current evidence gaps.

## Required repository checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
cargo test --workspace --all-features --doc --locked
python3 scripts/check_docs.py
python3 scripts/check_repo.py
python3 scripts/eval_test.py
cargo audit --deny warnings
cargo deny --all-features --locked check
cargo machete
cargo tree --duplicates --locked
```

`cargo-audit`, `cargo-deny`, and `cargo-machete` are additional development
tools. Install each with `cargo install --locked <tool>` if it is not already
available.

## Cross-architecture compile audit

CI runs strict Clippy and tests natively on Linux x86_64 and macOS aarch64. An
aarch64 contributor can compile-check x86_64-only kernels before pushing:

```sh
rustup target add --toolchain 1.96.1 x86_64-unknown-linux-gnu
cargo clippy --workspace --all-targets --all-features \
  --target x86_64-unknown-linux-gnu --locked -- -D warnings
```

This is a compile and lint gate, not an execution test. The Linux CI runner
executes the x86_64 test binaries on real x86_64 hardware.

## Test layers

1. Unit tests validate parsing, metadata, kernels, math, schemas, scheduler
   components, and edge conditions.
2. Integration tests validate crate boundaries, generated GGUF fixtures, CLI
   behavior, HTTP behavior, cancellation, caching, and batching.
3. Ignored real-model tests validate known GGUF artifacts and third-party client
   compatibility.
4. The eval harness records performance, memory, CPU, TTFT, latency, and token
   traces on a named machine.

## Real-model test variables

Ignored tests can use these paths:

```text
FERRITE_REAL_MODEL
FERRITE_REAL_TIER1_MODEL
FERRITE_QWEN_1_5B_Q6_MODEL
FERRITE_QWEN_1_5B_Q8_MODEL
FERRITE_SMOLLM_1_7B_Q4_MODEL
FERRITE_PHI3_MODEL
```

Run one ignored target explicitly after setting the matching variable. Avoid
`cargo test -- --ignored` across the entire workspace unless every required
artifact is present and the long-running cost is intentional. Use absolute
artifact paths for a package-wide run so every test resolves the same file
regardless of its process working directory.

## Eval harness

```sh
scripts/eval.sh --help
```

The harness builds locked release binaries, records the active Rust flags and
target directory, records the model SHA-256, runs CLI generation and precise
decode, optionally runs fixed engine batches, starts the HTTP server, drives a
streaming throughput client, samples RSS and CPU through `ps`, and writes JSON
plus Markdown to `scripts/evals/`. For the pinned default Qwen artifact, the
JSON also records source, revision, license, filename, size, SHA-256, license
URL, and download URL. It records request-cohort TTFT p50 and p95.
For one configured prompt, server parity verifies that every response has the
same complete ordered token-ID trace. For shared-prefix and distinct-prompt
workloads, it records and compares the exact trace for each corresponding
prompt between the default and continuous-batched routes. Equal token counts
alone do not pass the gate.

CPU sampling uses cumulative process CPU time from `ps`. If sample time is not
strictly increasing or the cumulative CPU counter moves backward within a
measurement window, the raw phase records `cpu_metrics_status`, omits CPU mean
and peak, and retains RSS. An invalid CPU window must not be interpreted as
zero utilization or used in an accepted CPU claim.

A minimal local run is:

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --generate-tokens 64 \
  --benchmark-runs 64
```

Select server prompt topology with `--server-workload identical`,
`shared-prefix`, `distinct`, or `mixed-length`. Each raw artifact covers one
topology. The throughput client also accepts repeatable `--prompt` values
directly and assigns requests to them in round-robin order.

For mixed-length cohorts, the harness pairs deterministic distinct prompts with
budgets from one token through the configured maximum. The throughput client
accepts repeatable `--max-tokens` values, validates every length-finished
response against its own budget, and reports the actual cohort completion-token
sum for aggregate throughput.

For optimization work, preserve both before and after artifacts. Compare the
same machine, model hash, prompt, generated-token count, benchmark length,
policy, worker count, build flags, and thermal conditions.

Important measurements require at least three clean repetitions. Reject runs
with material background load, thermal throttling, leaked processes, cache-state
mismatches, model-hash drift, or configuration drift. The harness records
provenance, but the operator remains responsible for declaring a contaminated
host instead of retaining misleading numbers.

For the complete identical, shared-prefix, distinct-prompt, and mixed-length
acceptance set, use the repeated suite:

```sh
scripts/eval_suite.py \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --server-batch-streams 4 \
  --requests 4 \
  --repetitions 3
```

The suite performs one locked release build, then passes `--skip-build` to each
child eval so compiler or security-scanner work cannot be reintroduced between
clean-host gates. A skipped-build child fails if any required release binary is
missing, and its artifact records `binary_build_mode=prebuilt`. Standalone
`scripts/eval.py` commands still build by default. The suite rejects preflight
load above its explicit per-core threshold, rejects leaked benchmark runtime
processes and reported macOS thermal pressure, fingerprints the source tree,
and produces ordinary raw eval artifacts for every repetition. Its final JSON
manifest accepts a case only when all repetitions have required metrics,
stable exact traces, and route parity. It reports min, median, and max for each
measured metric. Raw JSON, Markdown, and suite checkpoints use atomic
replacement and collision-safe names.

Child output is buffered during each case. The suite captures its postflight
host snapshot before emitting the complete report to the terminal, so terminal
rendering work cannot contaminate the background-process CPU gate it is meant
to enforce. Rejected preflight diagnostics are rate-limited while host sampling
continues at the configured poll interval, avoiding the same rendering
self-noise during a clean-window wait.

If a later case is contaminated after earlier cases completed cleanly, repeat
the identical command with `--resume-artifact PATH`. Schema version 2 pins the
source tree, host and Python identity, model paths and hashes, all three release
binary hashes, every ordered child command, every result-affecting setting, and
the clean-host policy. Resume rehashes each retained raw JSON artifact,
revalidates its exact tag, configuration, model hashes, preflight, and
postflight, and reuses only complete clean cases. Failed, partial, and
contaminated attempts remain in the new manifest. The previous manifest and raw
artifacts are never overwritten. Use `--skip-build` on a resumed invocation to
reuse the already-pinned release binaries; missing or changed binaries reject
the resume. Every invocation also monitors model and binary file identity before
and after each measured child so replacement during a long suite fails closed.

Use `--kernel-provider portable` on either harness to record a scalar-provider
parity cohort. Commands, provider selection, detected CPU features, and exact
token traces remain in the raw artifact.

Use `--threads N` on either harness when the evaluator must reserve CPU for
host services or when a comparison requires an explicit thread count. The
setting is propagated to every CLI and server phase, recorded in raw config and
commands, included in suite resume identity, and checked against the server's
reported runtime thread count. Keep the value identical across every repeated
case used by one performance claim.

Add `--server-soak-rounds 10` to repeat the identical-prompt cohort on each
already-loaded default and batched server. A soak run requires at least three
rounds. Before sampling, it runs one unmeasured cohort and waits for the same
configurable idle delay so lazy scheduler and allocator setup is part of the
steady-state baseline, not mistaken for a leak. The measured rounds revalidate
complete token traces, sample memory after every idle delay, and reject either
total growth or the range of the last half of samples above
`--server-soak-rss-tolerance-mib`. The default tolerance is 16 MiB and is
recorded in the artifact. The warm-up request count is recorded separately from
the measured soak request count.

Every platform records total process RSS. On macOS, clean read-only pages from
the memory-mapped model can enter or leave RSS under operating-system memory
pressure even when Ferrite has released its request and KV allocations. The
soak gate therefore also samples Apple's `phys_footprint` after every cohort
and uses that leak-sensitive physical footprint for the stable-range decision.
Raw RSS samples, RSS growth, RSS tail range, physical-footprint samples, the
selected gate metric, and its result all remain in the artifact. Failure to
obtain the macOS footprint fails the phase instead of silently falling back.
Other platforms continue to gate on RSS until an equally explicit native
private-footprint source is implemented.

The rejected
[`2026-07-14-143042`](../scripts/evals/2026-07-14-143042-qwen2.5-1.5b-instruct-q8_0-multi.md)
acceptance case motivated the explicit soak warm-up. Its Phi-3 batched physical
footprint stepped from 14,370,904 to 46,073,944 bytes, then moved only 98,304
bytes in the final sample. The focused
[`2026-07-14-145254`](../scripts/evals/2026-07-14-145254-phi-3-mini-4k-instruct-q4.md)
diagnostic kept the 16 MiB limit and passed after warm-up, with 5,832,704 bytes
of measured growth and a 5,718,016-byte tail range. This diagnostic establishes
the sampling method only; its timing is not clean-host performance evidence.

Later repeated attempts retained the symmetric tail gate and exposed a separate
25 to 35 MiB oscillation in identical-prompt Locus restore. ADR
[`0019`](adr/0019-borrowed-locus-snapshot-restore.md) records the borrowed
slice-to-mapping fix. The focused
[`2026-07-14-175626`](../scripts/evals/2026-07-14-175626-phi-3-mini-4k-instruct-q4.md)
post-change diagnostic reduced continuous-batched growth to 409,600 bytes and
tail range to 540,672 bytes without changing the threshold or exact token
traces. It is memory-method evidence, not a clean timing result.

The retained [bounded embedding-row diagnostic](benchmarks/2026-07-14-bounded-embedding-row-decode.md)
shows why both views are kept. A whole-matrix token-embedding decode created a
roughly 376 to 379 MiB private allocation, while clean mapped pages also moved
through total RSS. The source fix bounded row materialization; the physical
footprint gate verified the private allocation disappeared. That run was on a
busy host and supports no throughput or latency claim.

Use the bounded server block backend in a repeated evaluation with explicit
per-session sizing:

```sh
scripts/eval_suite.py \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --workload identical \
  --server-batch-streams 4 \
  --requests 4 \
  --server-soak-rounds 3 \
  --server-kv-backend locus \
  --server-kv-tokens-per-block 16 \
  --server-kv-max-tokens 128
```

Unless `--skip-build` is present, either eval harness builds the server with
the namespaced `ferrite-server/locus-kv` feature when Locus is selected. A
skipped build must already provide that feature-capable binary; otherwise the
child server fails closed. The token cap applies independently to every
admitted session and must cover the complete prompt plus worst-case decode KV
state. The backend, block size, cap, build feature, commands, binary hashes,
and resulting RSS samples remain part of the artifact identity.

Add `--server-prefix-cache` to both `scripts/eval.py` and
`scripts/eval_suite.py` when validating the unified cache and scheduler path.
The harness enables the bounded server cache, performs one unmeasured warm-up,
uses a workload-specific namespace, requests cache traces, and then measures
the same streaming cohort on the default and continuous-batched servers. Raw
artifacts retain the warm-up command, cached-token count, lookup kind, shared
prefix token count, exact output traces, TTFT, throughput, CPU, and RSS. Combine
this flag with at least three soak rounds for a cache-churn memory gate.
The warm-up sends exactly the first prompt and first token budget as one request;
the measured command still carries the complete configured cohort. The
standalone harness exits nonzero when any requested phase fails or default and
batched server traces disagree.

## llama.cpp reference comparison

`scripts/reference_compare.py` compares Ferrite with a pinned, CPU-only
`llama-server` build using the same GGUF bytes, prompt, generated-token budget,
thread count, and explicit greedy controls. Raw completion mode is the default.
It requires at least three clean repetitions and accepts an ordinary comparison
only when every exact content-associated token-ID pair matches, each runtime is
stable across repetitions, prompt and completion token counts match, finish
reasons match, and every run reaches the configured token budget.

Build llama.cpp separately at the pinned revision, with accelerators disabled
when CPU-only evidence is required, then run:

```sh
python3 scripts/reference_compare.py \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --llama-server target/llama.cpp/build-cpu/bin/llama-server \
  --threads 10 \
  --repetitions 3
```

If a later runtime is rejected after earlier repetitions completed cleanly,
repeat the identical command with `--resume-artifact PATH`. Schema version 2
pins the complete source-tree hash, both runtime binary hashes, host identity,
model path and hash, prompt, policy, llama.cpp revision and version, and every
result-affecting setting. Resume revalidates the recorded preflight and
postflight snapshots and retains only complete clean pairs. Partial or
contaminated attempts remain in the new artifact but are rerun with their fixed
repetition number and runtime order. The previous raw artifact is never
overwritten. Each in-progress JSON and Markdown checkpoint is replaced
atomically, and a filename collision receives a new suffix instead of reusing
either existing artifact. Model and runtime file identity is checked around
every measured process so mid-comparison replacement is rejected.

The script verifies the reported llama.cpp revision before starting, disables
prompt reuse and continuous batching, records both server commands and request
bodies, samples RSS and CPU, and writes raw JSON plus a Markdown summary under
`scripts/evals/`. A rejected comparison remains useful evidence and must not be
rewritten as parity. The endpoint and sampling fields follow the official
[llama.cpp server documentation](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md).

Chat comparison uses Ferrite's `/v1/chat/completions` route and llama.cpp's
documented `/apply-template` followed by `/completion` path. This preserves the
rendered prompt hash, prompt token count, exact output IDs, and llama.cpp's raw
runtime-emitted IDs. A terminal EOS ID that has no content is retained in the
raw runtime trace and excluded from the content-associated trace, matching the
OpenAI stream contract. The classifier also handles llama.cpp's split form,
where an empty EOS-token event immediately precedes a separate terminal event.
Chat-mode latency and RSS are diagnostic only because the reference side uses
two HTTP requests.

```sh
python3 scripts/reference_compare.py \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --request-mode chat \
  --prompt 'Rust is a systems programming language' \
  --max-tokens 1 \
  --threads 10 \
  --repetitions 3
```

Exact parity remains the default. `--allow-early-stop` permits both runtimes to
finish before the token ceiling only when their completion counts and finish
reasons match. `--numerical-policy PATH` can accept a mismatch only when one
reviewed policy case pins the model hash, prompt hash, request mode, token
budget, llama.cpp revision, rendered prompt hash, prompt count, finish reason,
and both exact traces. Ferrite's Qwen2.5 1.5B cases are recorded in
`scripts/numerical-policies/qwen2.5-1.5b-chat-near-ties-v1.json`. The policy is
not a global epsilon or token tolerance. Any unrecorded trace is rejected.

Clean-host preflight rejects elevated one-minute load, thermal pressure, leaked
inference runtimes, and any individual background process above the configured
`--max-background-process-cpu-percent` threshold. The default is 50 percent in
`ps` semantics, where 100 percent represents one fully occupied logical core.
Run `scripts/eval_suite.py --preflight-only` for an immediate JSON snapshot
without supplying a model or starting a build. A rejected snapshot exits
nonzero and lists every failed gate.
Every snapshot retains the top background CPU processes and the aggregate
observed process CPU percentage. Postflight repeats the background-process,
leaked-runtime, and thermal gates after the measured runtime exits. It ignores
only load average because the measured work necessarily remains in the trailing
one-minute value.
If process observation is unavailable, preflight and postflight fail closed.
Missing host telemetry is never interpreted as an idle machine.

When `scripts/eval.sh --download` acquires the default Qwen2.5 0.5B reference,
it uses a pinned Hugging Face revision and verifies the expected byte count and
SHA-256 before atomically publishing the file. Reports evaluating that exact
hash include the repository, revision, filename, size, hash, and source URL.

## Regression policy

- Correctness and API changes require deterministic tests.
- Unsafe changes require direct kernel parity tests and cross-architecture CI.
- Hot-path changes require a comparable eval and token trace.
- Cache or scheduler changes require cancellation, queueing, disconnect, and
  memory-bound tests. Retain repeated long-chat proof logs when queue or
  disconnect-recovery latency is part of the claim.
- Documentation claims require a command, test, ADR, benchmark, or source.

If throughput improves while token parity, TTFT, memory, or tail latency
regresses, record the tradeoff and do not call the change unconditionally
better.
