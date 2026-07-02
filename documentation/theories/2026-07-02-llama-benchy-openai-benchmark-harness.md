# Theory: llama-benchy OpenAI Benchmark Harness

Date: 2026-07-02

Status: Hypothesis

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

## Falsification Experiment

After the OpenAI-compatible server endpoint is available, run a small smoke
benchmark similar to:

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

Keep this as a benchmark-candidate theory until the OpenAI-compatible server
path is ready. Then run a minimal smoke against Ferrite and document the result
under `documentation/benchmarks` before adopting it as a standard performance
gate.
