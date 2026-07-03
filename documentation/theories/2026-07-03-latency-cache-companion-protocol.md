# Theory: Latency Cache Companion Protocol

Date: 2026-07-03

Status: Testing

## Hypothesis

Ferrite's internal long-chat gate and `llama-benchy` measure complementary
parts of the same OpenAI-compatible latency/cache problem:

- the Ferrite gate proves generated-context continuity, cache usage metadata,
  per-turn TTFT, RSS, reconnect/error behavior, and token-limit invariants;
- `llama-benchy` provides an external client view over prompt size, generation
  length, context depth, concurrency, prefix-caching mode, and JSON output.

Using both tools on the same model and token budgets should make cache-related
latency regressions easier to classify without conflating correctness proof
with benchmark trend data.

## Mechanism

The long-chat gate emits Ferrite-owned observability:

- `long_chat_result_assistant_context_hash`;
- `long_chat_result_generated_response_hash`;
- `long_chat_result_usage_prompt_tokens`;
- `long_chat_result_usage_cached_prompt_tokens`;
- `long_chat_result_prompt_cache_lookup`;
- `long_chat_result_time_to_first_token_ms`;
- `long_chat_result_server_rss_idle_bytes`;
- integrated summary fields for generated-context identity, reconnect probes,
  token IDs, RSS, and run completion.

`llama-benchy` emits client-side OpenAI benchmark data under the JSON
`benchmarks` array:

- `is_context_prefill_phase`;
- `context_size`;
- `prompt_size`;
- `response_size`;
- `concurrency`;
- `tg_throughput.mean`;
- `ttfr.mean`;
- `e2e_ttft.mean`.

The tools answer different questions. Ferrite's gate can explain whether a
turn was a cache miss, shared-prefix hit, or exact hit. `llama-benchy` can
stress common OpenAI-compatible prompt/depth/concurrency shapes and preserve
portable JSON for trend comparison.

## Current Evidence

The x86_64 Qwen 0.5B identity-summary proof showed cache depth dominating TTFT
inside a real generated-context conversation:

| Turn | Cached / Prompt | Lookup | TTFT ms | Context identity |
| ---: | ---: | --- | ---: | --- |
| 2 | 12 / 1054 | `shared_prefix_hit` | 382172 | turn 1 response became turn 2 context |
| 3 | 16 / 1054 | `shared_prefix_hit` | 371694 | turn 2 response became turn 3 context |
| 4 | 1054 / 1054 | `exact_hit` | 266 | turn 3 response became turn 4 context |

The same run reported:

```text
long_chat_summary_generated_context_identity_links=3
long_chat_summary_matching_generated_context_identity_links=3
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_run_complete=true
```

The existing local Qwen 0.5B `llama-benchy` prefix-cache matrix showed the
external client-side trend that larger prompt/depth combinations raise
end-to-end first-token time:

| Phase | Depth | Prompt | TG tok/s | TTFR ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| context | 256 | 1024 | 18.309952 | 2.309958 | 12364.619375 |
| inference | 256 | 1024 | 9.209281 | 1.343583 | 95545.705875 |
| context | 512 | 1024 | 15.108515 | 1.256959 | 27755.142417 |
| inference | 512 | 1024 | 8.136255 | 2.560209 | 126363.711750 |
| context | 1024 | 1024 | 10.615777 | 2.591125 | 69209.221375 |
| inference | 1024 | 1024 | 6.667127 | 2.782250 | 196352.914334 |

That matrix does not expose Ferrite's `cached_tokens` or generated-context
identity fields. It is therefore useful as an external latency companion, not
as a cache-correctness oracle.

## Extraction Commands

Ferrite gate summary extraction:

```sh
rg -n "long_chat_result=model|assistant_context_hash|generated_response_hash|usage_cached_prompt_tokens|usage_prompt_tokens|prompt_cache_lookup|time_to_first_token_ms|server_rss_idle_bytes|long_chat_summary" \
  target/proof/x86-qwen05-identity-summary-1024-2026-07-03/x86-qwen05-identity-summary-1024.log
```

`llama-benchy` JSON extraction:

```sh
jq -r '.benchmarks[] | [
  (.is_context_prefill_phase | if . then "context" else "inference" end),
  .context_size,
  .prompt_size,
  .response_size,
  .concurrency,
  .tg_throughput.mean,
  .ttfr.mean,
  .e2e_ttft.mean
] | @tsv' \
  documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-matrix.json
```

## Falsification Experiment

The theory is weakened if future paired runs show any of these outcomes:

- Ferrite gate reports exact prompt-cache hits while `llama-benchy` client-side
  e2e TTFT remains in the same range as known cache misses, after accounting for
  generation length and request shape.
- `llama-benchy` reports large TTFT improvements while direct Ferrite metadata
  shows no cached prompt tokens and no protocol or prompt-shape explanation.
- Repeated `llama-benchy` runs cannot be made repeatable enough to distinguish
  cache effects from run-to-run noise under the same model, prompt size, depth,
  concurrency, and server settings.

## Next Experiment

Run one bounded paired experiment on the current Qwen 0.5B tree:

1. Start Ferrite with `--experimental-prefix-cache`, explicit API key, and a
   fresh `prompt_cache_key`.
2. Run the Ferrite long-chat gate at `256`, `512`, and `1024` completion tokens
   with `--prompt-cache-trace`, generated-context identity summary, RSS
   sampling, error probe, and disconnect probe.
3. Run `llama-benchy` against the same server with:
   - `--pp 256 512 1024`;
   - `--tg 256 512 1024`;
   - `--depth 256 512 1024`;
   - `--enable-prefix-caching`;
   - `--extra-body prompt_cache_key=<same namespace family>`;
   - `--concurrency 1`;
   - `--latency-mode generation`;
   - `--format json`.
4. Archive raw JSON and proof logs under `target/proof/`, then write one
   benchmark note that explicitly separates:
   - correctness evidence from Ferrite's gate;
   - external latency trend evidence from `llama-benchy`;
   - RSS evidence from Ferrite/pod sampling;
   - unproven claims.

## Risks

- `llama-benchy --enable-prefix-caching` uses a system-message context-load
  phase. That is not the same shape as Ferrite's generated assistant
  long-chat context.
- `llama-benchy` measures client-observed OpenAI behavior and can be affected
  by streaming chunk shape, prompt adaptation, latency mode, and token ID
  availability.
- A 3x3x3 `pp/tg/depth` matrix is expensive on CPU. Keep the first paired run
  bounded or use a diagonal-only wrapper before attempting broad sweeps.
- Cache namespaces must be fresh per run family or explicitly documented, or
  prior cache state can contaminate the measurement.

## Decision Rule

Promote `llama-benchy` from companion to standard benchmark step only for
performance trend tracking. Do not use it to replace Ferrite's long-chat gate
until it can prove generated-context continuity, cache usage metadata,
reconnect/error behavior, stop/EOS behavior, and RSS invariants with the same
strength as the internal proof tool.

## First Paired Observation

A bounded local 256-token paired run now exists:

- Ferrite gate note:
  `documentation/benchmarks/2026-07-03-latency-cache-paired-qwen-0-5b-256.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-256.json`

The Ferrite gate proved generated-context identity and reconnect behavior, but
the generated-context lane did not converge to an exact prompt fixed point at
256 tokens. Follow-up turns reused only 12 to 14 prompt tokens and stayed in
`shared_prefix_hit`.

The companion `llama-benchy` run completed the different
system-context-prefix shape at depth 256, prompt 256, and generation 256. It
reported e2e TTFT of `11551.662375` ms for context load and `14724.025291` ms
for inference. That result is useful external latency evidence, but it does not
replace Ferrite's cache metadata or generated-context proof.

## Second Paired Observation

A bounded local 512-token paired run now exists:

- Ferrite gate note:
  `documentation/benchmarks/2026-07-03-latency-cache-paired-qwen-0-5b-512.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-512.json`

The Ferrite gate again proved generated-context identity and reconnect
behavior, but the generated-context lane did not converge to an exact prompt
fixed point at 512 tokens. All follow-up turns stayed in
`shared_prefix_hit`. Turn 3 reused a deeper shared prefix (`306 / 542`) and
TTFT dropped to `13743` ms, while turns 2 and 4 reused only `12 / 542` and
`20 / 542`, with TTFT near 27 seconds.

The companion `llama-benchy` run completed the system-context-prefix shape at
depth 512, prompt 512, and generation 512. It reported e2e TTFT of
`26284.783375` ms for context load and `38107.459042` ms for inference.

## Third Paired Observation

A bounded local 1024-token paired run now exists:

- Ferrite gate note:
  `documentation/benchmarks/2026-07-03-latency-cache-paired-qwen-0-5b-1024.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-paired-cache-1024.json`

The Ferrite gate reproduced the generated-context fixed-point mechanism inside
the paired protocol. Turns 2 and 3 were shallow shared-prefix hits (`12 / 1054`
and `16 / 1054`) with TTFT near 66 seconds. Turn 3 produced the same generated
response identity as its assistant context, and turn 4 became an exact prompt
hit with `1054 / 1054` cached prompt tokens and TTFT `230` ms.

The companion `llama-benchy` run completed the system-context-prefix shape at
depth 1024, prompt 1024, and generation 1024. It reported e2e TTFT of
`65689.654583` ms for context load and `114978.518583` ms for inference. The
local paired ladder now covers 256, 512, and 1024 tokens; x86_64 paired
validation remains open.

## First x86_64 Paired Observation

A bounded x86_64 256-token paired run now exists:

- Ferrite gate note:
  `documentation/benchmarks/2026-07-03-latency-cache-x86-paired-qwen-0-5b-256.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-256.json`

The Ferrite gate proved generated-context identity and reconnect behavior on a
bounded amd64 `staging` pod. The generated-context lane did not converge to an
exact prompt fixed point at 256 tokens. Follow-up turns reused only 12 to 14
prompt tokens and stayed in `shared_prefix_hit`; generated follow-up TTFT stayed
near 90 seconds.

The companion `llama-benchy` run completed the system-context-prefix shape via
a local port-forward to the x86 server. It reported e2e TTFT of
`87046.129750` ms for context load and `90266.089625` ms for inference. A first
port-forward attempt failed during the inference phase, so x86 companion runs
should record port-forward stability as part of the benchmark note.

## Second x86_64 Paired Observation

A bounded x86_64 512-token paired run now exists:

- Ferrite gate note:
  `documentation/benchmarks/2026-07-03-latency-cache-x86-paired-qwen-0-5b-512.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-512.json`

The Ferrite gate proved generated-context identity, error probe behavior, and
disconnect/reconnect behavior on the bounded amd64 `staging` pod. The
generated-context lane again did not converge to an exact prompt fixed point at
512 tokens. Follow-up turns stayed in `shared_prefix_hit`, but the reusable
prefix varied sharply: `12 / 542`, `306 / 542`, then `20 / 542`. TTFT followed
that prefix-depth shape: turns 2 and 4 stayed near 179 seconds, while turn 3
dropped to 83 seconds.

The companion `llama-benchy` run completed only after moving the client inside
the pod. Port-forward attempts were unstable for this long request shape. The
accepted in-pod benchmark reported e2e TTFT of `173297.980096` ms for context
load and `192137.629073` ms for inference with `--latency-mode none`. This
strengthens the protocol decision to keep `llama-benchy` as a companion trend
tool while Ferrite's gate remains the cache-correctness oracle.

The disconnected port-forward attempt also exposed a new theory candidate:
client cancellation may not immediately stop in-flight server generation. That
needs a focused cancellation/reconnect experiment with server-side CPU, RSS,
and request-lifetime evidence.

## Third x86_64 Paired Observation

A bounded x86_64 1024-token paired run now exists:

- Ferrite gate note:
  `documentation/benchmarks/2026-07-03-latency-cache-x86-paired-qwen-0-5b-1024.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-1024.json`

The Ferrite gate reproduced the fixed-point cache mechanism on the bounded
amd64 `staging` pod. Turns 2 and 3 were shallow shared-prefix hits:
`12 / 1054` and `16 / 1054` cached prompt tokens, both with TTFT near
369 seconds. Turn 3 generated the same response identity as its assistant
context, and turn 4 became an exact prompt hit with `1054 / 1054` cached prompt
tokens and TTFT `295` ms.

The accepted in-pod `llama-benchy` run completed the system-context-prefix
shape at depth 1024, prompt 1024, and generation 1024. It reported e2e TTFT of
`359177.165560` ms for context load and `420147.330409` ms for inference with
`--latency-mode none`.

This completes the x86_64 Qwen 0.5B paired 256/512/1024 ladder. It strengthens
the companion protocol because the external benchmark trend and Ferrite's
internal cache trace now both exist for all three diagonal x86 lengths, while
still preserving the boundary that only Ferrite's gate proves generated-context
identity, cached-token metadata, reconnect/error behavior, and RSS invariants.

## Lifecycle Companion Observation

A lifecycle-instrumented local companion run now exists:

- benchmark note:
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion.md`
- `llama-benchy` JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-256.json`
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-512.json`
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-lifecycle-companion-1024.json`

This run repeats the diagonal `256`, `512`, and `1024`
prompt/depth/generation shape against the current lifecycle-logging server.
`llama-benchy` reported e2e TTFT of `11682.690542` ms and `14931.817000` ms
for 256-token context and inference phases, `26427.955375` ms and
`39347.656125` ms for 512-token phases, and `66191.389000` ms and
`117547.157792` ms for 1024-token phases.

Ferrite's server lifecycle log recorded the matching long streams as completed
with no disconnect point. The 1024 context-load stream generated 1024 token IDs
in `189889` ms, and the 1024 inference stream generated 1024 token IDs in
`292133` ms.

The observation strengthens the benchmark/tool split. `llama-benchy` can be
kept as a standard external latency companion for OpenAI-compatible traffic,
but the long-chat gate still owns correctness proof for generated context,
cached-token metadata, reconnect/error handling, RSS sampling, and stop/EOS
behavior.
