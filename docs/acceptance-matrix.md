# Acceptance matrix

This matrix defines the evidence Ferrite needs before a correctness,
performance, model, or platform claim can be promoted. `scripts/eval.sh` is the
source of truth for measured results. Deterministic tests and the long-chat gate
provide focused evidence that the performance harness should not duplicate.

Status labels have precise meanings:

- **Automated**: a repository command produces pass or fail evidence now.
- **Artifact-gated**: the gate exists, but a local model file is required.
- **Manual**: the procedure is defined, but the result is not collected by the
  main harness.
- **Gap**: no sufficient gate exists yet. Do not make the corresponding claim.

## Non-negotiable acceptance rules

1. Compare the same GGUF bytes, SHA-256, prompt set, output budget, thread
   count, sampling policy, build profile, runtime settings, and machine.
2. Require exact generated token-ID parity unless a reviewed numerical policy
   explicitly defines another reference.
3. Retain at least three clean repetitions for important measurements. Report
   the median and the spread, never only the best result.
4. Reject a run affected by material background load, thermal throttling,
   a background process above the configured CPU threshold, leaked processes,
   a warm versus cold cache mismatch, inconsistent model hashes, or changed
   configuration.
5. Preserve raw JSON and Markdown output under `scripts/evals/`. Never commit a
   model file.
6. Treat every performance result as specific to its model hash, machine, and
   configuration.

## Model matrix

| Model class | Gate | Status |
| --- | --- | --- |
| SmolLM2 135M Instruct Q4_K_M fixture | `FERRITE_REAL_MODEL` client and HTTP ignored tests plus a retained native Linux x86_64 four-case HTTP pass | Artifact-gated |
| Qwen2.5 0.5B Instruct Q4_K_M reference | `FERRITE_REAL_TIER1_MODEL` plus `scripts/eval.sh --model PATH` | Artifact-gated |
| Phi-3 Mini 4K Instruct 3.8B Q4_K_M | Pinned built-in acquisition, exact tokenizer, one-token llama.cpp parity, and `FERRITE_PHI3_MODEL` native-EOG HTTP gate; clean performance still requires `scripts/eval.sh --model PATH` | Artifact-gated |
| Qwen 1.5B Q6_K and Q8_0 | `FERRITE_QWEN_1_5B_Q6_MODEL` and `FERRITE_QWEN_1_5B_Q8_MODEL` ignored tests | Artifact-gated |
| SmolLM 1.7B Q4 | `FERRITE_SMOLLM_1_7B_Q4_MODEL` ignored tests | Artifact-gated |

The harness accepts repeatable `--model` arguments and records each model's
SHA-256. Model paths are local inputs, not repository artifacts.

The local correctness inventory used these exact artifact identities. License
identifiers come from the metadata at each pinned source revision.

| Artifact | Source revision | License | Filename | Bytes | SHA-256 |
| --- | --- | --- | --- | ---: | --- |
| SmolLM2 135M Instruct Q4_K_M | `bartowski/SmolLM2-135M-Instruct-GGUF@09816acd5d99df7be770d85ea30822623dab342c` | `Apache-2.0` | `SmolLM2-135M-Instruct-Q4_K_M.gguf` | 105454432 | `2e8040ceae7815abe0dcb3540b9995eaa1fa0d2ca9e797d0a635ae4433c68c2d` |
| Qwen2.5 0.5B Instruct Q4_K_M | `Qwen/Qwen2.5-0.5B-Instruct-GGUF@df5bf01389a39c743ab467d734bf501681e041c5` | `Apache-2.0` | `qwen2.5-0.5b-instruct-q4_k_m.gguf` | 491400032 | `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db` |
| Qwen2.5 1.5B Instruct Q6_K | `Qwen/Qwen2.5-1.5B-Instruct-GGUF@91cad51170dc346986eccefdc2dd33a9da36ead9` | `Apache-2.0` | `qwen2.5-1.5b-instruct-q6_k.gguf` | 1464178720 | `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7` |
| Qwen2.5 1.5B Instruct Q8_0 | `Qwen/Qwen2.5-1.5B-Instruct-GGUF@91cad51170dc346986eccefdc2dd33a9da36ead9` | `Apache-2.0` | `qwen2.5-1.5b-instruct-q8_0.gguf` | 1894532128 | `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8` |
| SmolLM2 1.7B Instruct Q4_K_M | `HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF@2d4a76a30b4af41ecd395c35725ac11688d4cfe4` | `Apache-2.0` | `smollm2-1.7b-instruct-q4_k_m.gguf` | 1055609536 | `decd2598bc2c8ed08c19adc3c8fdd461ee19ed5708679d1c54ef54a5a30d4f33` |
| Phi-3 Mini 4K Instruct Q4 | `microsoft/Phi-3-mini-4k-instruct-gguf@a64113399c2f6b8ad3e11c394733a2ddadaa7f33` | `MIT` | `Phi-3-mini-4k-instruct-q4.gguf` | 2393231072 | `8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef` |

With all six real-model environment variables set to absolute paths, this
command discovered 69 cases and passed all 69 serially on 2026-07-14:

```sh
cargo test --release --locked -p ferrite-server --all-features --tests \
  -- --ignored --test-threads=1
```

The set covers clients, raw and chat HTTP, streaming, stop sequences, model
catalog, queue admission, concurrent rejection and waiting, long output, and
request-rate paths across the 135M, 0.5B, 1.5B Q6/Q8, 1.7B, and Phi-3
artifacts. This is correctness evidence, not clean performance evidence.

The Phi-3 correctness check used the registry SHA-256
`8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef`.
For the same 12-token rendered prompt, Ferrite's portable and automatic kernel
providers selected token ID 7521 and the same ordered top-five token IDs as the
pinned llama.cpp reference. This is correctness evidence only. No throughput,
TTFT, or memory claim is promoted without clean repeated eval artifacts.

The dedicated `openai_real_phi3` release test rehashes that exact artifact and
checks non-streaming plus streaming Chat Completions. For the prompt `Write one
word about iron.`, Ferrite emits visible token IDs 2443 and 295, then terminal
token ID 32007 (`<|end|>`). The response reports 10 prompt tokens, three
completion tokens, `finish_reason=stop`, and visible content ` Steel` without
leaking the terminal control token. The pinned llama.cpp run used the same
10-token rendered prompt and emitted 2443, 295, 29889, then terminal 32007.
The multi-token traces are therefore documented as different, not promoted as
exact parity. The test proves deterministic useful execution and correct
model-native termination, not comparative performance.

## Workload matrix

| Workload | Gate | Status |
| --- | --- | --- |
| Single-stream generation | CLI and default server phases in `scripts/eval.sh` | Automated |
| Identical concurrent prompts | `--server-workload identical --server-batch-streams N --requests N` | Automated |
| Shared prefix, distinct suffixes | `--server-workload shared-prefix --server-batch-streams N --requests N` | Automated |
| Completely distinct prompts | `--server-workload distinct --server-batch-streams N --requests N` | Automated |
| Multi-turn conversations | `ferrite-openai-long-chat-gate` and ignored real-model tests | Artifact-gated |
| Short and long output sequences | `--generate-tokens N` and long-chat `--token-lengths` | Automated or artifact-gated |
| 2K and 8K context cases | Source-generated GGUF sessions execute through both exact boundaries; real-model long-context runs remain separate | Automated or artifact-gated |
| Cancellation, disconnect, and queue pressure | Long-chat disconnect and queue probes plus server integration tests | Automated or artifact-gated |
| Mixed output lengths in one concurrent cohort | `--server-workload mixed-length` pairs distinct prompts with per-request budgets | Automated |
| Long-running churn and memory return | `--server-soak-rounds N` establishes steady state with one unmeasured cohort, then repeats exact measured cohorts and gates idle growth plus tail range; artifacts retain RSS, and macOS additionally gates on physical footprint to separate clean mapped model pages | Automated |
| Prefix reuse inside continuous batching | `--server-prefix-cache` prewarms one namespace, traces longest-prefix hits, and compares exact default-versus-batched token traces | Automated |

`scalar_context_limit::executes_two_and_eight_k_token_context_boundaries`
builds source-controlled GGUF fixtures, loads them through the normal parser and
model adapter, evaluates every position through attention, RoPE, and KV state,
then verifies deterministic exhaustion at positions 2,048 and 8,192. This does
not replace a retained long-context run on a useful real model.

The throughput client accepts repeatable `--prompt` values. Requests use them in
round-robin order. For one configured prompt it verifies that every response has
the same complete ordered token-ID trace. For multiple prompts it records one
exact trace per prompt, verifies repeated uses of each prompt are stable, and
lets the eval harness compare corresponding traces between the default and
continuous-batched routes.

`--max-tokens` is also repeatable. When prompts and token budgets are both
repeated, their counts must match and each request receives the corresponding
pair in round-robin order. Cohort throughput uses the actual sum of completion
tokens, so short jobs cannot be counted as if they reached the largest budget.

A comparable acceptance pass uses separate invocations so every raw artifact
has one unambiguous prompt topology. The suite enforces at least three
repetitions, waits for a clean host, balances case order across repetitions,
checks that source files do not change during the run, verifies exact token
parity, and writes median plus observed-range summaries:

```sh
scripts/eval_suite.py \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --server-batch-streams 4 \
  --requests 4 \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --repetitions 3 \
  --tag-prefix acceptance
```

The retained schema-v2 manifest
`scripts/evals/2026-07-14-085536-acceptance-suite.json` accepted this complete
matrix on an Apple M5 Pro after three clean repetitions per case. It pins
source tree
`a2b6bfa7b659e283de239029a80fa8c0a3581f1d812bc34a74d35823c0cb8651`,
the Qwen2.5 0.5B Q4_K_M SHA-256 listed above, and all three release binary
hashes. The configuration used 64 generated tokens, 64 decode benchmark steps,
four requests, four continuous-batch streams, CLI batches of four and eight,
the automatic kernel provider, the prefix cache, three soak rounds, a 500 ms
idle delay, and a 16 MiB RSS tolerance. The accepted medians and observed
ranges are specific to those exact identities and settings:

| Case | Aggregate or decode tok/s, median [min, max] | TTFT, median [min, max] | Peak or post-load RSS, median [min, max] bytes |
| --- | ---: | ---: | ---: |
| CLI single-stream decode | 87.73 [86.50, 88.13] | 0.150 [0.146, 0.156] s | 584073216 [583991296, 584105984] |
| CLI batch 4 | 145.71 [128.12, 145.72] | not separately measured | 591396864 [588431360, 594526208] |
| CLI batch 8 | 172.63 [156.13, 172.95] | not separately measured | 597786624 [592183296, 603684864] |
| Server identical prompts | 143.18 [137.56, 144.71] | 2 [2, 2] ms | 597704704 [588775424, 601260032] |
| Server shared prefix | 122.72 [112.68, 124.09] | 235 [229, 311] ms | 591314944 [591069184, 595640320] |
| Server distinct prompts | 115.42 [114.70, 115.84] | 400 [397, 402] ms | 597377024 [593526784, 599654400] |
| Server mixed length | 78.42 [77.45, 78.86] | 422 [420, 422] ms | 598261760 [593788928, 598687744] |

All default and continuous-batched soak checks passed in all three identical
prompt repetitions. For the continuous-batched route, total idle RSS growth
had median -32014336 bytes with range [-50790400, 16384], and the last-half
idle RSS range had median 49152 bytes with range [16384, 3457024]. These values
are below the fixed 16 MiB gate and are not a general memory bound.

For shared-prefix, distinct, and mixed-length cases, different prompts are
expected to produce different traces. Acceptance requires each prompt's trace
to remain stable and the corresponding default and continuous-batched traces
to match exactly. A global all-traces-equal flag is therefore not the gate for
those cases. The mixed-length aggregate uses the actual 113 completion tokens
from budgets 1, 16, 32, and 64. It does not extrapolate the one-token request
to the full cohort.

One selected CLI batch-4 run recorded a zero cumulative CPU mean alongside a
782 percent interval peak. That contradictory CPU window is treated as an
invalid sampler observation and is excluded from every promoted claim above.
The harness now reports `cpu_metrics_status=cumulative_counter_regressed` and
omits CPU mean and peak when a cumulative counter moves backward. RSS remains
available from the same samples.

Use `--dry-run` to inspect the balanced command sequence without building or
measuring. Use `scripts/eval_suite.py --preflight-only` to print one
machine-readable clean-host snapshot without a model or build. The command
exits nonzero when the host is rejected. Preflight also rejects an individual
background process above
`--max-background-process-cpu-percent`, which defaults to 50 percent where 100
percent means one fully occupied logical core. The manifest records this
threshold, load threshold, polling configuration, aggregate observed process
CPU, and top CPU processes. Postflight repeats the process, leaked-runtime,
and thermal checks. It intentionally omits load average because the measured
runtime itself remains represented in that trailing average.
If the host cannot provide process observations, the gate rejects the run
instead of treating missing data as a clean machine.

## Platform matrix

| Platform | Gate | Status |
| --- | --- | --- |
| Apple Silicon | Native CI tests, local aarch64 tests, and local real-model eval | Automated or artifact-gated |
| Linux x86_64 | Native GitHub Actions tests plus retained homelab command evidence | Automated or manual |
| Linux ARM64 correctness | Native GitHub Actions Clippy and all-feature tests | Automated |
| Linux ARM64 real-model performance | No retained native real-model eval | Gap |
| macOS x86_64 | Native Intel GitHub Actions tests | Automated |
| Windows x86_64 correctness | Native GitHub Actions Clippy and default plus all-feature tests | Automated |
| Windows x86_64 release and performance | No native release archive or retained real-model eval | Gap |
| Rosetta x86_64 on Apple Silicon | Focused provider parity test under x86_64 target | Automated |

A cross-target compile proves that conditional code compiles. It does not prove
runtime instruction dispatch, numerical parity, or performance on native
hardware.

The 2026-07-14 homelab snapshot documented in
[`portability.md`](portability.md#platform-evidence) passed strict workspace
Clippy and the full all-target, all-feature workspace test command on an Intel
N100 under native Linux x86_64. Its post-test storage eviction is recorded
separately. This is native correctness evidence, not real-model or performance
evidence. A separate bounded pod then passed all four release-mode HTTP tests on
the exact pinned SmolLM2 135M Q4_K_M artifact. That extends native correctness
evidence to one real model, but remains neither larger-model nor performance
evidence. The retained
`scripts/evals/2026-07-14-085037-phi3-native-linux-x86-http.json` artifact adds
release-mode non-streaming and streaming HTTP correctness for the exact pinned
Phi-3 Mini 3.8B Q4 artifact on the same Intel N100. It does not add a native
performance claim.

## Comparison matrix

| Reference | Required identity | Status |
| --- | --- | --- |
| Ferrite default versus continuous batching | Same Ferrite build, model hash, per-prompt token traces, and runtime settings | Automated |
| Ferrite versus llama.cpp | `scripts/reference_compare.py` pins the same GGUF bytes, raw prompt, token budget, thread count, and greedy settings | Artifact-gated |
| Ferrite chat versus llama.cpp template reference | Same messages, rendered-prompt hash and count, exact content-associated traces, raw llama.cpp trace, finish reason, and greedy settings | Artifact-gated |

The reference comparator verifies the pinned llama.cpp revision, disables
prompt-cache reuse and continuous batching, alternates runtime order, requires
three clean repetitions, and retains exact traces plus latency and process
measurements. In chat mode, llama.cpp `/apply-template` supplies the auditable
prompt for `/completion`; timing and RSS are diagnostic because this is a
two-request correctness path.

Exact token parity remains the default. The reviewed policy at
`scripts/numerical-policies/qwen2.5-1.5b-chat-near-ties-v1.json` accepts only
two exact Qwen2.5 1.5B trace identities. It does not define a global tolerance.
The Q6 first-token case reproduced Ferrite token 9454 and llama.cpp token 49 in
three clean repetitions and was accepted under the policy. The Q8 long-chat
case reproduced stable `help` and `assist` traces in three no-policy
repetitions. A later policy run exposed that llama.cpp emits its terminal EOS
ID in an empty event immediately before its separate terminal event. The
comparator now retains that ID in the raw runtime trace, excludes it from the
content-associated trace, and has a split-event regression test.

The accepted schema-v2 Q8 artifact,
`2026-07-14-062429-qwen2.5-1.5b-instruct-q8_0-chat-llama-cpp-reference.json`,
retains three clean, order-balanced pairs selected from 15 attempts across a
12-artifact resume chain. Repetitions 1, 2, and 3 selected attempts 4, 6, and 5
respectively. Every selected runtime has a clean preflight and postflight, both
runtimes are trace-stable, prompt and completion counts match, finish reasons
match, and all three pairs reproduce the exact reviewed Q8 policy identities.
The artifact pins source tree
`0ad64c972cf33004577e5dbbc21d23cb03d48800aec03656cd1f56c67ff6e03f`,
model SHA-256
`d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`,
Ferrite binary
`cfce3116099e46b5bbe873d4f179bf17b874dc3d5fbe680876dbd5aeaddbf15a`,
llama.cpp binary
`6cac476d991456828f28638759245ca8c28cf733486a2efa27d8a8f33e029147`,
revision `6eddde06a4f25d55d538b5d15628dcc2b6882147`, and the Apple M5 Pro host
identity. Contaminated attempts remain raw evidence but are excluded from the
summary. The original artifacts were not overwritten during resume. This is
accepted comparative correctness under the bounded policy. Chat-mode TTFT,
throughput, and RSS remain diagnostic and do not promote a performance claim.

## Sampling and template matrix

| Behavior | Gate | Status |
| --- | --- | --- |
| Exact default-greedy preservation | Inference, CLI, runtime, and HTTP token-trace tests | Automated |
| Positive-temperature seeded repeatability | Sampler and runtime isolation tests plus CLI and HTTP integration tests | Automated |
| Top-k, top-p, min-p, repetition, frequency, presence, and logit bias | Focused sampler, validation, CLI, and route tests | Automated |
| Seed independence from unrelated requests | Interleaved sampler and runtime tests | Automated |
| Sampled streaming | SSE route tests with explicit temperature and seed | Automated |
| Qwen2.5 model-provided ChatML | Exact source-controlled template fixture plus local reference GGUF smoke | Artifact-gated |
| Llama 3 and Llama 2 recognized template families | Source-controlled representative template fixtures | Automated |
| Arbitrary or unknown Jinja | Bounded fallback tests; arbitrary execution is intentionally unsupported | Automated |
| Greedy throughput regression | Three clean `scripts/eval_suite.py` repetitions on an idle host | Artifact-gated |

Implementation tests prove control flow and deterministic behavior. They do
not replace a clean, real-model performance comparison. Keep the performance
claim artifact-gated until the required repeated run is uncontaminated.

## Structured output and compatibility matrix

| Behavior | Gate | Status |
| --- | --- | --- |
| Chat JSON-object syntax | Prefix-parser properties, adversarial unit cases, and HTTP generation tests | Automated |
| JSON Schema constrained output | No schema compiler or grammar gate | Gap |
| Qwen ChatML function definitions and parsed calls | Definition bounds, history validation, parser, response-shape, and route tests | Automated |
| Tool execution boundary | Tests retain calls as response data; caller owns authorization and execution | Automated |
| Non-streaming Responses text input | Request bounds, generation, usage, cache, auth, CORS, and response-shape tests | Automated |
| Responses streaming, state, tools, and multimodal input | Explicit structured rejection tests | Automated |
| HTTP body exhaustion | Explicit 2 MiB pre-deserialization limit and oversized-body route test | Automated |

The JSON grammar caps output at 1 MiB and 64 levels. Tool schemas and arguments
are separately bounded by bytes, nesting, node count, definition count, and
parsed-call count. These are hostile-input reliability gates, not claims of
hosted OpenAI feature parity.

## Reliability and security matrix

| Risk | Gate | Status |
| --- | --- | --- |
| Malformed or truncated GGUF | Every truncated byte prefix of a valid fixture, malformed headers, bounded counts, nested metadata, strings, tensor shapes, offsets, and ranges | Automated |
| Tokenizer edge cases | BPE, SPM, byte fallback, control-token, invalid-merge, partial UTF-8, and cancellation tests | Automated |
| Chat-template failures and limits | Recognized-family, unsupported-role, oversized-template fallback, and 16 MiB rendered-prompt rejection tests | Automated |
| Long context and empty input | Exact 2K and 8K fixture execution and exhaustion plus HTTP empty-input validation; real-model long-context generation remains model-gated | Automated or artifact-gated |
| Cancellation and connection loss | Tokenization, prompt-layer polling, stream-body drop, and live TCP disconnect tests; latency distributions remain model-gated | Automated or artifact-gated |
| Queue exhaustion and backpressure | Scheduler capacity, immediate rejection, bounded waiting, queue-order, and long-chat probes | Automated or artifact-gated |
| Bounded allocation exhaustion | Fallible GGUF and prompt reservations plus Locus out-of-block rollback and no-unbounded-fallback tests | Automated |
| Whole-matrix embedding materialization | Bounded Q4_K, Q5_K, and Q6_K row-window implementation, crossing-block tests, and retained full-model footprint diagnostic | Automated or artifact-gated |
| Global process allocator exhaustion | No allocator-fault injection harness or process-level recovery guarantee | Gap |
| Concurrency and cache isolation | Parallel route, scheduler namespace, cache fingerprint, churn, eviction, and lease-lifetime tests | Automated |
| Long-running memory leaks | Token-stable server soak with idle RSS growth and tail-range gates | Artifact-gated |
| Unsupported CPU features | Provider fallback, capability detection, portable parity, native CI, and cross-target strict Clippy | Automated |
| Hostile HTTP input | Authentication before parsing, malformed requests, bounded schemas, explicit 2 MiB body limit, and oversized-body rejection | Automated |

The process-allocation gap is explicit because bounded input and KV paths reduce
exposure but do not make a general Rust process recoverable after system memory
exhaustion.

## Advanced performance gate

Speculative decoding is not eligible for implementation until the sampling,
scheduler, KV, portability, clean throughput, TTFT, and steady-state memory
gates all have comparable retained artifacts. ADR 0017 records that those
artifacts were incomplete when the decision was made. A later accepted
baseline does not authorize speculation by itself; draft-model, prompt or
n-gram candidates must still pass acceptance-rate, fallback, sampling,
realistic-workload, and extra-memory gates. This is a gate decision, not a
negative performance result.

## Measurement matrix

| Measurement | Current evidence | Status |
| --- | --- | --- |
| Exact generated token IDs | CLI trace, per-request cohort trace, and default-versus-batched per-prompt parity | Automated |
| TTFT p50 and p95 | Throughput-client request cohort percentiles | Automated |
| Inter-token latency | First response min, p50, p95, and max | Automated |
| Per-stream and aggregate throughput | CLI, engine batch, and HTTP request cohort metrics | Automated |
| Peak RSS | CLI and server process sampling | Automated |
| Steady-state memory after churn | Server soak records every idle RSS sample; macOS also records and gates on physical footprint | Automated |
| CPU utilization | CLI and server process sampling | Automated |
| Energy use | No portable collector | Gap |
| Queue delay and disconnect recovery latency | Long-chat probes report client-observed admission, first generated event, completion, and reconnect attempts; repeated clean proof logs are required for distributions | Artifact-gated |
| Isolated cancellation cleanup latency | `openai_stream_lifecycle` records disconnect stage, observation time, and server-side `disconnect_to_finish_ms`; repeated real-model logs are still required for distributions | Artifact-gated |
| Provenance | Model hash, commit, dirty state, build profile, host, CPU, Rust version, commands, and runtime config | Automated |

The matrix is intentionally stricter than the current implementation. A gap is
a boundary on claims and a prioritized engineering target, not evidence of
failure in an unrelated supported path.
