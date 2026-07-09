# Ferrite Evaluation Script — Design

**Date:** 2026-07-09
**Status:** Approved
**Deliverable:** `scripts/eval.py` — a repeatable, single-command evaluation harness for tracking Ferrite performance over time on the local machine.

## Purpose

Give Ferrite a one-command evaluation run that measures, per model artifact:

- **Throughput** — decode tokens/second (both wall-clock streamed and in-process precise).
- **Time to first token (TTFT)** — prefill latency, separated from model load time.
- **Memory** — post-load RSS, peak RSS, plus Ferrite's own accounting (`model_file_bytes`, `scalar_weight_bytes`, `kv_cache_bytes`).
- **CPU** — mean and peak `%cpu` during generation.
- **Server-path metrics** — end-to-end TTFT, streaming tokens/second, and token-latency percentiles as an OpenAI client sees them.

Each run writes a machine-diffable JSON record and a human-readable Markdown report to `scripts/evals/`, so improvements (or regressions) are trackable commit over commit.

## Tool choice

Python 3, **stdlib only** (no pip installs). Python orchestrates; all measurement of inference itself stays inside Ferrite's already-instrumented Rust binaries (`ferrite` CLI `key=value` output, `ferrite-openai-throughput` streaming metrics), so interpreter overhead does not pollute the numbers. Wall-clock line timestamps and `ps` sampling are the only measurements taken by Python, and both are dominated by process behavior, not Python speed.

Alternatives rejected:

- **bash + awk** — SSE handling, JSON emission, and percentile math get unmaintainable.
- **Rust `ferrite-eval` crate** — fits the workspace ethos but couples eval orchestration to the strict deny-lint build and slows iteration; the workspace already provides the measurement binaries.

## Interface

```sh
scripts/eval.py [--model <path.gguf>]... [options]
```

- `--model` (repeatable): model artifacts to evaluate. Default: scan `target/models/*.gguf`.
- If no models are found: offer (interactive y/N; `--download` to pre-approve, `--no-download` to forbid) to download the small reference artifact **Qwen2.5-0.5B-Instruct-Q4_K_M (~400 MB)** into `target/models/`. Larger models are always placed manually — keeps with the "no big downloads on the local Mac" rule.
- `--generate-tokens N` (default 64), `--benchmark-runs N` (default 64), `--prompt <text>` (default: fixed prompt so runs are comparable), `--requests N` server requests (default 4).
- `--skip-server` / `--skip-cli` to run one phase only.
- `--tag <label>` free-form label recorded in the output (e.g. `locus-kv`, `pre-simd-fix`).

## Run flow

1. **Setup**
   - `cargo build --release -p ferrite-cli -p ferrite-server` (binaries: `target/release/ferrite`, `target/release/ferrite-server`, `target/release/ferrite-openai-throughput`).
   - Capture environment: git commit + dirty flag, branch, rustc version, CPU brand string, physical/logical cores, RAM, OS version, hostname.

2. **CLI phase** (per model)
   - **Generation run:** spawn
     `ferrite --model M --prompt P --sleep-after-load-ms 2000 --generate-tokens N --stream`,
     timestamp every stdout line, and sample `ps -o rss=,%cpu= -p <pid>` every 100 ms for the process lifetime.
     - *Load time* = spawn → `sleep_after_load_ms=` marker line (fs read + GGUF parse + tokenizer + weight load).
     - *Post-load RSS* = RSS samples inside the sleep window (the project's established sampling method).
     - *TTFT (prefill)* = sleep end → first `stream_token_id=` line. Note: the first generated token is produced by prefill itself, so this is prefill + argmax.
     - *Streamed decode tok/s* = inter-token deltas across `stream_token_id=` lines: mean, p50, p95, and overall tokens/elapsed.
     - *Peak RSS*, *mean/peak %cpu* over the generation window.
     - Parse `model_file_bytes`, `scalar_weight_bytes`, `kv_cache_bytes`, `generated_stopped_on_eos`.
   - **Precise decode run:** `ferrite --model M --prompt P --benchmark-runs N` → `benchmark_avg_ns` → canonical in-process decode tok/s (no pipe/print overhead).

3. **Server phase** (per model)
   - Start `target/release/ferrite-server --model M --model-id eval --bind 127.0.0.1:<free port> --api-key ferrite-eval --hard-max-tokens <N+headroom> --default-max-tokens N`.
   - Poll `GET /health` until ready (timeout → mark phase failed, kill, continue).
   - Run `ferrite-openai-throughput` against `/v1/chat/completions` with `--stream`, `--requests R` (sequential; the engine holds a single inference permit) → parse `streaming_time_to_first_token_ms`, `streaming_tokens_per_second`, `streaming_token_latency_p50/p95_ms`, `requests_per_second`, usage token counts.
   - Sample the **server** process RSS/%cpu during the request window.
   - Always terminate the server (finally-block), even on failure.

4. **Output**
   - `scripts/evals/<YYYY-MM-DD-HHMMSS>-<model-stem>.json` — versioned schema (`schema_version: 1`) with `env`, `config`, and per-model `cli` / `server` metric blocks, including the exact command lines run (perf claims need full context).
   - Sibling `.md` — summary tables in the style of `documentation/benchmarks/`.
   - The same summary table printed to the terminal.
   - Multiple models in one run → one file pair per run containing all models (run timestamp names the file; model stems listed inside; filename uses first model stem + `-multi` suffix when >1).

## Error handling

- No model + download declined → exit 2 with the manual download hint (URL + target path).
- Build failure → exit with cargo's output; nothing written.
- CLI run mismatch/crash → record phase `"status": "failed"` with stderr excerpt; still write the file (a failed run is evidence too).
- Server never healthy → kill process group, record failure, continue with CLI results.
- Ctrl-C → best-effort child cleanup via `finally`/signal handling.

## Testing

- `scripts/eval_test.py` (stdlib `unittest`): key=value parser, percentile math, ps-sample aggregation, JSON/Markdown emitters (golden-ish assertions on structure, not exact floats).
- End-to-end verification: one real run on this machine against the downloaded reference model; the produced JSON/MD checked by hand and kept as the first record in `scripts/evals/`.

## Non-goals

- No quality/accuracy eval (perplexity, task scores) — parity vs llama.cpp is covered elsewhere.
- No concurrency/load testing — the engine serves one generation at a time by design today.
- No trend dashboard — the JSON schema exists so one can be built later.
