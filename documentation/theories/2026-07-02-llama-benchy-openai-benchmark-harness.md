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

This moves the theory from pure hypothesis to early compatibility evidence. It
does not validate the full 256/512/1024-token protocol, prefix caching,
concurrency behavior, RSS behavior, reconnect/error behavior, or stop/EOS
behavior.

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

Run one bounded 256-token `llama-benchy` baseline against the same
`Qwen2.5-0.5B-Instruct-Q4_K_M` model and compare it with the nearest Ferrite
long-chat timing output. Only after that should the protocol expand to the full
256/512/1024-token matrix.

Do not adopt this as a standard gate until the result is compared with
Ferrite's own long-chat timing output for the same model and token lengths.
