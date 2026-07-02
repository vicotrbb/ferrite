# Benchmark Protocol: llama-benchy OpenAI Harness

Date: 2026-07-02

## Purpose

Define the first bounded `llama-benchy` protocol for Ferrite's
OpenAI-compatible HTTP server. This is a protocol note, not a completed
benchmark result.

This protocol tests whether an external OpenAI-compatible benchmark can produce
repeatable prompt-processing, decode-throughput, first-token, concurrency, and
prefix-cache measurements that are comparable with Ferrite's own long-chat gate
artifacts.

## Source

- Tool: <https://github.com/eugr/llama-benchy>
- Observed capability: OpenAI-compatible `/v1/chat/completions` benchmarking
  with configurable prompt tokens, generated tokens, context depth, concurrency,
  latency mode, prefix-caching mode, JSON/CSV/Markdown output, and progress
  JSONL.
- Constraint: It supplements Ferrite's long-chat gate. It does not prove
  repeated generated-context conversations, reconnect behavior, malformed
  request handling, stop/EOS correctness, or server RSS behavior by itself.

## Preconditions

- Ferrite server exposes `POST /v1/chat/completions` and `GET /v1/models`.
- Server is started with one Tier 1 model and a stable model id.
- Run on the same host or same staging pod class used by the matching Ferrite
  long-chat notes.
- Record the Ferrite commit SHA, server command, model path, quantization, CPU,
  CPU limits, memory limits, and raw output path before interpreting results.
- Use a fresh result filename for every run. Do not overwrite prior benchmark
  artifacts.

## Baseline Smoke

Use this first to check protocol compatibility without prefix-cache assumptions:

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
  --save-result documentation/benchmarks/YYYY-MM-DD-llama-benchy-baseline.json
```

Minimum acceptance:

- command exits `0`;
- JSON result file is written;
- results include prompt-processing and token-generation rows for 256, 512,
  and 1024 token cases;
- rows include first-response or first-token timing fields usable for comparison
  with Ferrite long-chat gate notes;
- no Ferrite-specific patch is required in `llama-benchy`.

## Prefix-Cache Experiment

Use this only against a Ferrite server started with `--experimental-prefix-cache`:

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
  --save-result documentation/benchmarks/YYYY-MM-DD-llama-benchy-prefix.json
```

Minimum acceptance:

- command exits `0`;
- JSON result file is written;
- context-load rows and follow-up prompt rows are both present;
- Ferrite usage metadata reports non-zero cached prompt tokens on at least one
  generated follow-up request;
- the result can be compared to a matching Ferrite long-chat run that used the
  same model, token lengths, and `prompt_cache_key`.

## Concurrency Step

After the baseline smoke works, run one small concurrency step:

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:8000/v1 \
  --model ferrite-local \
  --served-model-name ferrite-local \
  --pp 512 \
  --tg 256 \
  --concurrency 1 2 4 \
  --latency-mode generation \
  --format json \
  --save-result documentation/benchmarks/YYYY-MM-DD-llama-benchy-concurrency.json
```

Minimum acceptance:

- command exits `0`;
- total throughput and per-request throughput are distinguishable;
- server logs show bounded queue behavior, not unbounded memory growth;
- a matching RSS note records before, during, after, and idle memory samples.

## Result Note Requirement

Every executed `llama-benchy` run must be paired with a Markdown result note in
`documentation/benchmarks/` that records:

- exact Ferrite commit SHA;
- exact `llama-benchy` invocation and version source;
- server command and model id;
- raw result file path;
- prompt-processing throughput;
- decode throughput;
- first-response and first-token timing;
- concurrency level;
- RSS evidence source;
- whether prefix-cache fields were present;
- comparison against the nearest Ferrite long-chat gate result;
- explicit unproven scope.

## Current Status

A minimal compatibility smoke has been executed against Ferrite:

- Result note:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke.md`
- Failed pre-change raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke.json`
- Successful post-change raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-compat-smoke-after-return-token-ids.json`
- Token-id streaming smoke note:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.md`
- Token-id streaming raw result:
  `documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-token-ids-smoke.json`

That smoke used `--pp 32`, `--tg 16`, one run, concurrency `1`, no warmup, no
coherence check, and no prompt adaptation. It proves external tool
compatibility only.

A follow-up no-stop streaming smoke used `--pp 16`, `--tg 8`, one run,
concurrency `1`, no warmup, no coherence check, and no prompt adaptation after
Ferrite started returning token IDs on chat content chunks. The command exited
`0` and did not print the previous `No token_ids in response, using local
tokenization` fallback line.

The full protocol has not been executed. The next proof slice is a bounded
256-token baseline against one available Tier 1 model, followed by a result
note comparing `llama-benchy` output with the nearest Ferrite long-chat timing
artifact.
