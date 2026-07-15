# Operational tools

Ferrite includes two focused server clients in addition to `scripts/eval.sh`.
They are built with the `ferrite-server` package and print machine-readable
`key=value` output.

## Throughput client

`ferrite-openai-throughput` drives completion or chat-completion requests and
reports request rate, streaming timing, token rate, finish reason, usage, token
ID coverage, text identity, request-cohort TTFT p50 and p95, and optional server
RSS.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:8080 \
  --endpoint chat-completions \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'Write one sentence about iron.' \
  --requests 4 \
  --concurrency 4 \
  --max-tokens 64 \
  --stream \
  --stream-usage \
  --api-key local-secret
```

Use `--rss-pid <server-pid>` to sample server RSS before, during, and after the
request window. Use `--prompt-cache-key` and `--prompt-cache-trace` only when
evaluating the experimental prefix cache.

`--prompt` is repeatable. The client assigns requests to configured prompts in
round-robin order and requires at least one request per configured prompt. It
prints exact per-prompt token-ID traces and whether repeated uses of each prompt
were stable. This supports identical, shared-prefix, and fully distinct prompt
cohorts without treating expected cross-prompt output differences as failures.

`--max-tokens` is repeatable too. Repeated prompt and token-budget lists are
paired by index and assigned round-robin. The client validates each
length-finished response against its assigned budget and prints aggregate
cohort usage totals, which are the correct numerator for mixed-length
throughput.

## Long-chat gate

`ferrite-openai-long-chat-gate` validates multi-turn continuity, finish
sources, cache accounting, error recovery, client disconnects, queue recovery,
and proof artifact output. The queue probe reports client-observed contender
admission latency, time to first generated event, and total elapsed time. The
disconnect probe reports reconnect attempts plus the time from disconnect to
reconnect admission, first generated event, and completion.

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --queue-probe \
  --require-probes error,disconnect,queue \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 256,512,1024 \
  --addr 127.0.0.1:8080 \
  --api-key local-secret
```

The gate can write a proof log and final exit code with `--proof-log` and
`--proof-exit-code`. Generated context caps, state capsule placement, required
finish sources, and required response substrings are available for controlled
long-context experiments. Treat one probe as a behavioral observation, not a
latency distribution. Repeat clean runs and retain every proof log before
reporting percentiles. Disconnect recovery includes cancellation cleanup,
connection setup, queueing, and generation, so it is not an isolated measure of
server-side cancellation cleanup.

## Metadata commands

All four executable entry points support `--help`, `-h`, `--version`, and `-V`
without requiring a model or starting a server.

## Complete evaluation

Prefer [`scripts/eval.sh`](evaluation.md#eval-harness) for release comparisons.
It orchestrates the CLI, server, throughput client, resource sampling, token
parity, and JSON plus Markdown output in one reproducible command.

Use `scripts/eval_suite.py` when a claim requires the full prompt-topology
matrix and at least three clean repetitions. It groups ordinary eval artifacts
in a machine-readable acceptance manifest and reports median plus range instead
of selecting a best run. The suite performs one locked release build and makes
its child evals reuse those verified binaries, avoiding duplicate Cargo work
between clean-host checks. Standalone `scripts/eval.py` runs continue to build
unless the operator explicitly supplies `--skip-build` with existing binaries.
Raw eval artifacts and suite manifests use atomic replacement and do not reuse
an existing filename.

A rejected schema-v2 suite manifest can be supplied back through
`--resume-artifact`. Resume requires exact source, binary, host, model, ordered
case, configuration, and clean-host policy identity. It rehashes and revalidates
every retained raw case artifact, selects only complete cases with clean
preflight and postflight evidence plus passing route parity and required
metrics, retains all failed or contaminated attempts, and writes a new manifest
instead of mutating the previous one.

Run `scripts/eval_suite.py --preflight-only` to inspect the clean-host policy
and one live JSON snapshot without supplying a model or starting a build. A
rejected host exits nonzero.

Pass `--server-soak-rounds N` to the suite to add repeated exact-trace and idle
memory-stability gates to the identical-prompt default and batched server
cases. Each route runs one unmeasured cohort before the measured idle samples,
and records its warm-up request count separately. Total RSS is always retained.
On macOS the gate also records and uses Apple's physical footprint so clean
memory-mapped model pages cannot masquerade as retained KV memory.

Pass these flags to run those server cases against an explicitly bounded block
pool:

```sh
--server-kv-backend locus \
  --server-kv-tokens-per-block N \
  --server-kv-max-tokens N
```

The suite enables the namespaced server Cargo feature when it builds, records
all three settings, and propagates them only to server children. CLI
measurements retain their independently recorded KV backend.

Use `scripts/reference_compare.py` for the separate pinned llama.cpp
comparison. It holds model bytes, raw prompt, token budget, thread count, and
greedy controls constant, records exact token IDs, and rejects mismatched or
unstable traces. Chat mode records llama.cpp's rendered prompt hash and raw
runtime-emitted IDs before comparing content-associated traces. This includes
the split llama.cpp form where an empty EOS-token event precedes its terminal
event. A numerical policy is opt-in and can accept only one fully pinned,
reviewed trace identity.

A rejected schema-v2 reference artifact can be supplied back through
`--resume-artifact`. Resume requires exact source, executable, host, model,
prompt, policy, revision, and configuration identity. It selects only complete
pairs whose stored preflight and postflight snapshots still pass the recorded
policy, retains every contaminated attempt, and writes a new artifact rather
than mutating the previous one. Checkpoint replacement is atomic, and existing
artifact names are never reused.

Both repeated harnesses reject elevated load, thermal pressure, leaked
inference processes, and an individual background process above the configured
CPU threshold before launching a measured runtime. After each runtime exits,
they repeat the process, leak, and thermal checks while ignoring only the
trailing load average produced by the measurement itself.
Both gates fail closed when host process observation is unavailable.
