# Performance golden path

This is the recommended path for high local CPU performance. It distinguishes
portable defaults from experimental machine-specific optimizations and requires
measurement before an optimization is kept.

## 1. Establish a clean baseline

Close unrelated CPU-heavy applications, connect power on laptops, keep thermal
conditions stable, and record the exact commit and machine. Confirm a clean
tree:

```sh
git status --short --branch
```

Build the locked release graph:

```sh
cargo build --release --locked -p ferrite-cli -p ferrite-server
```

Ferrite's release profile uses optimization level 3, ThinLTO, one codegen unit,
panic abort, and stripped symbols. This profile reduced the measured CLI binary
from 1.4 MiB to 795 KiB while retaining the exact benchmark token trace. Its
interleaved throughput comparison remained within normal run-to-run noise.

Do not use debug binaries for performance decisions. Do not compare runs from
different prompts, token counts, models, quantizations, build flags, or thermal
states as if they were equivalent.

## 2. Let Ferrite select the worker count

Start without `--threads`. Ferrite chooses a topology-aware count for the
selected execution policy. An explicit value is useful for controlled sweeps,
but a larger count can reduce throughput when memory bandwidth is saturated.

To test a fixed count:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write a short story about a rusty robot who learns to sail.' \
  --benchmark-runs 128 \
  --threads 7
```

Record the median of repeated runs, not the single best result.

## 3. Use the exact path as the compatibility baseline

The default execution policy is the exact path. It is the baseline for token
parity and is the correct first choice on every supported CPU:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write a short story about a rusty robot who learns to sail.' \
  --benchmark-runs 128
```

To isolate dispatch from model semantics, repeat a correctness run with
`--kernel-provider portable`. This is a diagnostic oracle, not a recommended
performance setting.

## 4. Opt into residual I8MM only on supported Arm CPUs

On aarch64 CPUs with FEAT_I8MM, the residual activation path can reduce decode
cost. It is approximate at the matrix level, even though the current parity
suite has no generated token divergence. Keep it opt-in:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write a short story about a rusty robot who learns to sail.' \
  --benchmark-runs 128 \
  --experimental-residual-q8-activation-matvec
```

The accepted Apple M5 Pro gate on 2026-07-10 reached 105.54 precise decode
tokens per second and 106.17 streamed tokens per second on Qwen2.5 0.5B
Q4_K_M, with seven workers. A separate 512-token run produced the same exact
token-trace hash for the default and residual policies. This is evidence for
that machine and artifact, not a universal guarantee. See the
[complete performance gate](benchmarks/2026-07-10-oss-quality-hardening.md).

The residual path cannot be combined with experimental continuous batching.
Ferrite rejects the combination instead of silently selecting another policy.

## 5. Choose latency or aggregate throughput

For one interactive request, use the default scheduler and, on validated Arm
hardware, consider residual I8MM.

For several simultaneous streaming requests, evaluate continuous batching:

```sh
target/release/ferrite-server \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --bind 127.0.0.1:8080 \
  --experimental-batched-decode \
  --max-batch-streams 4
```

Batching usually raises aggregate throughput while reducing each stream's
throughput. The accepted 2026-07-13 Apple M5 Pro gate reached a repeated median
of 131.45 aggregate completion tokens per second at four requests, 41.03% above
the initial 93.21 tok/s observation, and 159.58 tok/s at eight requests. Every
response in each cohort matched the default route's complete ordered token-ID
trace. The four-request median used 568.8 MiB peak RSS, 40.55% below the initial
956.8 MiB observation. See the
[complete gate](benchmarks/2026-07-13-memory-mapping-and-shared-prefill.md).

Those measurements use the eval harness's shared-prompt workload. Exact equal
prompts benefit from one prefill plus independent KV snapshot restoration;
distinct prompts still use batched context-only prefill but do not receive that
fan-out optimization.

## 6. Run the complete eval harness

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --experimental-residual-q8-activation-matvec \
  --batch-streams 2 \
  --batch-streams 4 \
  --batch-streams 8 \
  --skip-server \
  --tag golden-path-residual

scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --skip-cli \
  --server-batch-streams 4 \
  --requests 4 \
  --tag golden-path-server-batch
```

Residual I8MM and continuous batching are separate execution contracts, so the
harness evaluates them in separate invocations instead of silently changing a
requested policy.

The harness writes JSON and Markdown under `scripts/evals/` and records TTFT,
request-cohort TTFT p50 and p95, decode throughput, token latency, RSS, CPU,
commands, model SHA-256, complete ordered per-prompt server token-ID traces,
cohort parity, host, Rust version, commit, branch, and dirty-tree state.

## 7. Promotion rule for optimizations

Keep a performance change only when all of these conditions hold:

1. The comparison uses the same model, prompt, token count, policy, worker
   count, build settings, and stable machine conditions.
2. The repeated-run median improves. A best-case outlier is not evidence.
3. Generated token IDs match the required exact or prior reference trace.
4. Formatting, strict Clippy, tests, rustdoc, eval-harness tests, and dependency
   policy checks pass.
5. Memory, TTFT, per-stream latency, and aggregate throughput regressions are
   measured and explicitly accepted or rejected.

`RUSTFLAGS="-C target-cpu=native"` makes a binary specific to the build CPU.
The 2026-07-10 Apple M5 Pro comparison did not show a reliable improvement over
the portable ThinLTO build, so it is not part of the golden path. Treat it as a
separate experiment and retain it only when a comparable local eval proves a
benefit.
