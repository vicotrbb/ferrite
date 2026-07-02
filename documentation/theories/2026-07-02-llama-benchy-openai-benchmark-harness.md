# Theory: llama-benchy OpenAI Benchmark Harness

Date: 2026-07-02

Status: Testing

## Source Check

Source inspected: <https://github.com/eugr/llama-benchy>

Observed on 2026-07-02:

- The project describes itself as a `llama-bench`-style benchmark tool for
  OpenAI-compatible LLM endpoints.
- It evaluates `/v1/chat/completions`, with `/v1/models` used for model
  discovery when possible.
- It supports configurable prompt-processing tokens, generation tokens,
  context depths, runs, concurrency, latency measurement modes, exact generation
  lengths, JSON/CSV/Markdown output, and progress JSONL emission.
- Its prefix-caching mode performs a two-step context-load plus inference
  benchmark, which is relevant once Ferrite has real prefix reuse instead of
  key construction only.
- The latest GitHub release shown during this check was `v0.3.8`, published on
  2026-06-10.

The first bounded `llama-benchy` compatibility smoke has been executed against
Ferrite. The pre-change run reached Ferrite but failed because `llama-benchy`
0.3.8 sends `return_token_ids: true`; the post-change run completed one
streaming chat benchmark request after Ferrite accepted that typed extension.
See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke.md`.

Ferrite's own proof clients now expose `--prompt-cache-key` for
`chat-completions`, and the long-chat gate passes that through to the
throughput client. That makes direct comparison with `llama-benchy --extra-body
prompt_cache_key=...` practical without patching either side.

## Hypothesis

`llama-benchy` can become a useful external benchmark harness for Ferrite once
the OpenAI-compatible HTTP surface is stable enough for real chat-completion
runs.

It should supplement, not replace, Ferrite's dedicated long-chat proof gates.

## Mechanism

`llama-benchy` targets OpenAI-compatible endpoints and exercises
`/v1/chat/completions` with configurable prompt-processing token counts,
generation token counts, context depths, concurrency levels, and JSON/CSV
result output.

Those properties map well to Ferrite's current optimization questions:

- prompt prefill cost at 256, 512, and 1024 token budgets;
- decode throughput under fixed generation lengths;
- time to first response and end-to-end time to first content token;
- concurrency saturation for the HTTP server path;
- future prefix-cache impact using its prefix-caching mode;
- machine-readable benchmark artifacts under `documentation/benchmarks`.

## Expected Measurement

This theory is worth adopting if `llama-benchy` can run against Ferrite without
custom patches and produce repeatable JSON output for at least:

- one small Tier 1 model through Ferrite's OpenAI-compatible server;
- `--pp 256 512 1024`;
- `--tg 256 512 1024`;
- `--concurrency 1`, then a small load step such as `2` or `4`;
- `--latency-mode generation`;
- a saved JSON or CSV result artifact.

The first useful result should report prefill throughput, decode throughput,
TTFR, estimated prompt-processing time, and end-to-end first-token time in a
format that can be compared against Ferrite's existing long-chat benchmark
notes.

## First Compatibility Result

On 2026-07-02, `llama-benchy 0.3.8` completed a minimal smoke against
`Qwen2.5-0.5B-Instruct-Q4_K_M` served through Ferrite:

- `--pp 32`
- `--tg 16`
- `--runs 1`
- `--concurrency 1`
- `--latency-mode none`

The result file recorded numeric prompt-processing, decode-throughput, TTFR,
estimated prompt-processing time, and end-to-end first-token fields. During the
run, `llama-benchy` reported that Ferrite did not return token IDs and used
local tokenization instead.

After Ferrite added chat stream `token_ids` on no-stop content chunks, a second
minimal smoke completed with:

- `--pp 16`
- `--tg 8`
- `--runs 1`
- `--concurrency 1`
- `--latency-mode none`

The command exited `0`, wrote
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.json`,
and did not print the previous local-tokenization fallback line. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.md`.

Ferrite's own OpenAI throughput and long-chat proof clients now summarize token
ID coverage for no-stop streaming chunks. A local Qwen 0.5B smoke reported
`streaming_content_chunks=8`, `streaming_token_id_chunks=8`,
`streaming_token_ids=8`, and
`streaming_all_content_chunks_have_token_ids=true`. See
`documentation/benchmarks/2026-07-02-openai-throughput-qwen-0-5b-token-id-observability.md`.

A bounded 256-token baseline has now run against the same local Qwen 0.5B
server. The generation-latency run reported `tg_throughput.mean` of
`17.108893401529688` and `e2e_ttft.mean` of `11560.583166981814`, but
`pp_throughput` was null. A companion latency-none run reported
`pp_throughput.mean` of `294450.6291659168` and `tg_throughput.mean` of
`16.579741995461383`. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-256-baseline.md`.

The same bounded baseline has also run at 512 tokens. The generation-latency
run reported `tg_throughput.mean` of `12.495702857387348` and `e2e_ttft.mean`
of `27288.6513749836`, but `pp_throughput` was null. The companion latency-none
run reported `pp_throughput.mean` of `570285.7620520669` and
`tg_throughput.mean` of `12.425758423855232`. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-512-baseline.md`.

The bounded baseline has also run at 1024 tokens. The generation-latency run
reported `tg_throughput.mean` of `8.071044619458204` and `e2e_ttft.mean` of
`69507.34637497226`, but `pp_throughput` was null. The companion latency-none
run reported `pp_throughput.mean` of `1128453.4269715385` and
`tg_throughput.mean` of `8.053815994759002`. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-1024-baseline.md`.

A small concurrency step has also run with `--pp 512`, `--tg 256`, and
`--concurrency 1 2 4`. The default zero-wait server returned HTTP 429 under
excess concurrency. With `--inference-wait-ms 300000`, the run completed at all
three concurrency levels and showed queued single-permit behavior: total
`tg_throughput.mean` dropped from `13.520923091874657` at concurrency 1 to
`6.460806574464915` at concurrency 4, while per-request `tg_req_throughput.mean`
stayed around `13.6`. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-concurrency.md`.

A bounded prefix-cache smoke has also run with `--pp 128`, `--tg 32`,
`--depth 128`, `--enable-prefix-caching`, and an explicit
`prompt_cache_key`. `llama-benchy` completed its context-load and inference
rows without tool-specific patches. The context-load row reported
`tg_throughput.mean` of `21.219811746997483`, and the inference row reported
`tg_throughput.mean` of `18.137211810417345`. A direct Ferrite probe against
the same server configuration then sent two identical requests with the same
cache key; the second response reported `cached_tokens=17` and dropped from
`1065.8981669985224` ms to `386.81875000474975` ms. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-smoke.md`.

A larger prefix-cache matrix has now run with prompt sizes 256, 512, and 1024,
context depths 256, 512, and 1024, and a fixed 32 generated tokens. The
list-form command produced a 3x3 cross-product with context-load and inference
rows for each combination. The largest inference row, depth 1024 and prompt
1024, reported `tg_throughput.mean=6.667126615243654` and
`e2e_ttft.mean=196352.9143340129`. Companion direct Ferrite probes confirmed
exact-prompt cache hits at larger prompt-token scales: 342, 678, and 1350
cached prompt tokens on the second request. See
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-matrix.md`.

This moves the theory from pure hypothesis to working external benchmark
evidence. It does not validate generated-context long-chat cache reuse,
high-concurrency serving, reconnect/error behavior under load, or stop/EOS
behavior under load.

## Shared-Prefix Follow-Up

The upstream `llama-benchy` repository still presents the tool as a
`llama-bench`-style benchmark for OpenAI-compatible endpoints, with prompt
processing, token generation, context-depth, latency, output-format, and
concurrency controls. That matches the next Ferrite benchmarking need.

After Ferrite's shared-prefix cache gate completed on the internal long-chat
proof client, `llama-benchy` should be used as an external comparison harness,
not as the primary correctness gate. The internal gate directly validates
generated-context turn sequencing, `cached_prompt_tokens`, RSS sampling,
streaming token IDs, unauthorized reconnect, and disconnect/reconnect behavior.
`llama-benchy` is better suited for repeated throughput and latency sweeps once
the correctness gate is green.

Next external benchmark candidate:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model smollm2-135m-q4_k_m \
  --served-model-name smollm2-135m-q4_k_m \
  --pp 256 512 1024 \
  --tg 256 512 1024 \
  --depth 256 512 1024 \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:benchy:shared-prefix \
  --concurrency 1 \
  --latency-mode generation \
  --format json \
  --save-result documentation/benchmarks/YYYY-MM-DD-llama-benchy-shared-prefix.json
```

Adoption criterion: use it only after the Ferrite long-chat gate passes the
same token budget, and document both artifacts together so correctness and
benchmark throughput do not get conflated.

## Shared-Prefix Smoke

After the Ferrite shared-prefix long-chat gates completed on the internal proof
client, a bounded `llama-benchy` prefix-cache smoke ran against
`SmolLM2-135M-Instruct-Q4_K_M`. The benchmark note is
`documentation/benchmarks/2026-07-02-llama-benchy-smollm-135m-shared-prefix-smoke.md`.

The run used:

- `--pp 64`
- `--tg 16`
- `--depth 64`
- `--enable-prefix-caching`
- `--extra-body prompt_cache_key=ferrite:benchy:smollm135:shared-prefix`
- `--latency-mode generation`

`llama-benchy` exited `0`, wrote JSON output, and reported the expected
context-load and inference rows. A companion direct Ferrite probe sent two
divergent prompts with the same cache key; the second prompt reported
`cached_tokens=3`, proving a shared-prefix cache hit through Ferrite's own
OpenAI-compatible usage metadata.

This reinforces the tool split: use Ferrite's internal long-chat gate for
correctness and protocol proof, and use `llama-benchy` for external benchmark
trends after correctness is already established.

## Generated-Context Windowing Fit

The 512-token x86_64 generated-context window probe completed with Ferrite's own
long-chat gate because that proof needed reconnect/error coverage, RSS sampling,
generated follow-up turn validation, stop/finish behavior, and streaming token
ID invariants. The benchmark note is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-token-window-probed-512.md`.

`llama-benchy` remains useful for the next comparison layer: once a candidate
windowing policy is exposed through Ferrite's OpenAI-compatible request path,
run it as an external throughput and context-depth harness. Do not use it as the
correctness oracle for windowing, because it does not replace Ferrite's
long-chat continuity, reconnect/error, stop/EOS, and RSS gates.

## 128-Token Calibration

A bounded 128-token calibration now compares `llama-benchy` with Ferrite's own
OpenAI throughput client on the same local Qwen 0.5B server. The benchmark note
is
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-128-calibration.md`.

The Ferrite throughput client reported `20.673809` streamed tokens/sec for a
single 128-token chat-completions request. `llama-benchy` reported
`20.101652` generated tokens/sec in generation-latency mode and `20.176806`
generated tokens/sec in latency-none mode for `--pp 128 --tg 128`.

This supports using `llama-benchy` for external decode-throughput trend checks.
It also reinforces that first-token timings need careful interpretation:
Ferrite's direct comparator used a short 15-token prompt, while `llama-benchy`
forced a 128-token prompt.

## Falsification Experiment

Run a small no-cache baseline first:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:8000/v1 \
  --model ferrite-local \
  --served-model-name ferrite-local \
  --pp 256 512 1024 \
  --tg 256 512 1024 \
  --concurrency 1 \
  --latency-mode generation \
  --format json \
  --save-result documentation/benchmarks/YYYY-MM-DD-llama-benchy-smoke.json
```

Then run an explicit namespace/cache experiment against a Ferrite server started
with `--experimental-prefix-cache`:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:8000/v1 \
  --model ferrite-local \
  --served-model-name ferrite-local \
  --pp 256 512 1024 \
  --tg 256 512 1024 \
  --depth 256 512 1024 \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:benchy:prefix-smoke \
  --concurrency 1 \
  --latency-mode generation \
  --format json \
  --save-result documentation/benchmarks/YYYY-MM-DD-llama-benchy-prefix-smoke.json
```

The theory is falsified for near-term use if Ferrite's API semantics require
tool-specific patches, if streaming chunks do not provide usable timing data,
or if the benchmark cannot generate stable artifacts comparable to the existing
long-chat proof notes.

## Risks

- `llama-benchy` currently evaluates `/v1/chat/completions`; it will not cover a
  separate completions-only path unless Ferrite exposes compatible chat
  semantics.
- It does not replace repeated multi-turn conversation validation, client
  reconnect behavior, server-side error behavior, or stop/EOS proof.
- Its prefix-caching mode may not match Ferrite's eventual cache-key semantics,
  especially if Ferrite requires explicit cache namespaces or token-exact
  identity.
- HuggingFace tokenizer selection must match the loaded GGUF tokenizer closely
  enough for prompt-size adaptation to be meaningful.

## Next Step

Build a generated-context long-chat cache gate or a small diagonal wrapper for
repeatable prefix-cache regression checks. Keep the 3x3 `llama-benchy`
cross-product as an explicit long benchmark because it is too heavy for a quick
default gate on the local Mac.

Do not adopt this as a standard gate until the result is compared with
Ferrite's own long-chat timing output for the same model and token lengths.
