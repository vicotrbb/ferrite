# 2026-07-09 — Evaluation harness (`scripts/eval.py`)

## What was built

A one-command, stdlib-only Python 3 evaluation harness plus unit tests:

- `scripts/eval.py` — orchestrates a release build, a CLI generation run, a
  precise in-process decode benchmark, and an OpenAI-server run driven by
  `ferrite-openai-throughput`; writes a versioned JSON record
  (`schema_version: 1`) and a Markdown summary to `scripts/evals/`.
- `scripts/eval_test.py` — stdlib `unittest` suite (14 tests) covering the
  pure logic: key=value parsing, nearest-rank percentiles, `ps` cputime
  parsing, sample-window aggregation, CLI timing metrics from a synthetic
  timeline, output naming, and Markdown rendering.

Design spec: `docs/superpowers/specs/2026-07-09-eval-script-design.md`.
Plan: `docs/superpowers/plans/2026-07-09-eval-script.md`.

## How measurements are taken

- All inference measurement stays inside Ferrite's instrumented binaries;
  Python only timestamps stdout lines and samples `ps`.
- **Load time** = spawn → `sleep_after_load_ms=` marker (printed + flushed
  after weight load, before the pause).
- **TTFT (prefill)** = end of the post-load pause → `prompt_token_ids=`
  (first line printed after prefill). Model load is excluded and reported
  separately.
- **Decode tok/s (precise)** = `1e9 / benchmark_avg_ns` from
  `--benchmark-runs` (in-process, no pipe overhead).
- **Decode tok/s (streamed)** = inter-token deltas of `stream_token_id=`
  lines (wall clock, includes print/pipe overhead).
- **Memory** = `ps -o rss=` sampled every 100 ms; post-load RSS is the peak
  inside the pause window (the method used by earlier benchmark notes).
- **CPU%** = deltas of `ps -o cputime=` over wall time (true utilization,
  not ps's decaying `%cpu` average).
- **Server metrics** = `ferrite-openai-throughput --stream --stream-usage`
  against `/v1/chat/completions` on an ephemeral port (TTFT, streamed
  tok/s, token latency p50/p95), with the server process sampled for
  RSS/CPU during the request window.

## First baseline (this machine)

Record: `scripts/evals/2026-07-09-185239-qwen2.5-0.5b-instruct-q4_k_m.{json,md}`

Apple M5 Pro (15 cores, 24 GiB), macOS 26.5.2, rustc 1.96.0,
commit 3ec7b08 (dirty: untracked eval outputs), Qwen2.5-0.5B-Instruct
Q4_K_M, prompt "Write a short story about a rusty robot who learns to
sail.", 64 generated tokens, 64 benchmark runs, 4 server requests:

| metric | value |
| --- | --- |
| load | 0.761 s |
| TTFT (prefill, load excluded) | 0.368 s |
| decode tok/s (precise / streamed) | 35.98 / 37.13 |
| CLI token latency p50 / p95 | 26.8 / 29.6 ms |
| RSS post-load / peak | 1013.0 / 1018.1 MiB |
| CPU mean / peak (generation) | 757.2 / 793.1 % |
| server TTFT | 508 ms |
| server streamed tok/s | 28.48 |
| server token latency p50 / p95 | 27 / 31 ms |
| server RSS peak | 1020.5 MiB |

## Caveats

- TTFT is defined as prefill excluding load; server TTFT (508 ms) includes
  HTTP + chat-template prefill for an 18-token prompt.
- Streamed tok/s includes pipe/print overhead; the precise in-process
  number is canonical for decode-throughput claims.
- `ps` sampling at 100 ms can miss sub-100 ms RSS spikes.
- RSS (~1013 MiB) is roughly 2× the model file (468.6 MiB): retained
  scalar/quantized weights plus allocator overhead; consistent with the
  Tier-0/Tier-1 memory-accounting notes.
- The auto-download offer covers only the ~400 MB Qwen2.5-0.5B Q4_K_M
  reference; larger artifacts are placed in `target/models/` manually.

## How to run

```sh
python3 scripts/eval.py                 # scan target/models/*.gguf, both phases
python3 scripts/eval.py --model target/models/foo.gguf --tag my-experiment
python3 scripts/eval.py --skip-server   # CLI phase only
python3 scripts/eval_test.py            # unit tests
```
