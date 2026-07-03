# Benchmark: llama-benchy Qwen 0.5B Current Smoke

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run a current-commit `llama-benchy` smoke against Ferrite's OpenAI-compatible
`/v1/chat/completions` endpoint after the latest long-chat identity work.

This keeps the external benchmark harness warm for future theory testing. It
does not replace Ferrite's long-chat gate because it does not validate repeated
generated-context turns, reconnect behavior, RSS sampling, or Ferrite-specific
prompt-cache usage metadata.

## Environment

- Ferrite commit: `096ad60`
- Host: local macOS workspace
- Server: `127.0.0.1:18204`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>
- Server binary SHA256:
  `3fd89b31ff30a89ae3e0a999b2db8ca8e96d2f36afe844b5d495a216a97de19e`
- Captured stdout:
  `target/proof/llama-benchy-qwen-0-5b-current-smoke.stdout.txt`
- Raw JSON:
  `documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-current-smoke.json`

The local server was stopped after the run. A final bind-specific process check
returned no process.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Tokenizer passed to `llama-benchy`: `Qwen/Qwen2.5-0.5B-Instruct`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18204 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64 \
  --inference-wait-ms 30000
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18204/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 32 \
  --tg 16 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --format json \
  --save-result documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-current-smoke.json
```

The command exited `0` and wrote:

```text
17 target/proof/llama-benchy-qwen-0-5b-current-smoke.stdout.txt
82 documentation/benchmarks/2026-07-03-llama-benchy-qwen-0-5b-current-smoke.json
```

## Result

| Metric | Value |
| --- | ---: |
| `version` | `0.3.8` |
| `latency_mode` | `none` |
| `concurrency` | 1 |
| `context_size` | 0 |
| `prompt_size` | 32 |
| `response_size` | 16 |
| `pp_throughput.mean` | 11156.142363 |
| `tg_throughput.mean` | 24.522112 |
| `peak_throughput.mean` | 26.156919 |
| `ttfr.mean` | 2.868375 ms |
| `est_ppt.mean` | 2.868375 ms |
| `e2e_ttft.mean` | 1805.328458 ms |

## Interpretation

The external benchmark harness still works against the current Ferrite
OpenAI-compatible chat endpoint. This is useful for future experiments because
`llama-benchy` can sweep prompt size, generation length, context depth,
prefix-cache mode, and concurrency while saving JSON.

For Ferrite release proof, this harness should stay paired with Ferrite's own
long-chat gate. `llama-benchy` does not emit Ferrite's
`usage.prompt_tokens_details.cached_tokens` or `ferrite_cache` trace fields,
and this smoke does not exercise reconnect, error handling, RSS, or stop/EOS
behavior.

## Limits

This run does not prove:

- 256, 512, or 1024-token behavior;
- prefix-cache behavior;
- concurrency behavior;
- x86_64 behavior;
- steady-state memory behavior;
- stop/EOS behavior.
