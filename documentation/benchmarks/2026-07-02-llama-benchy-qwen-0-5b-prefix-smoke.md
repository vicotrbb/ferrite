# Benchmark: llama-benchy Qwen 0.5B Prefix-Cache Smoke

Date: 2026-07-02

## Purpose

Run the first bounded `llama-benchy` prefix-cache smoke against Ferrite's
OpenAI-compatible `/v1/chat/completions` server and pair it with direct
Ferrite usage-metadata proof.

This is a compatibility and behavior smoke. It is not the full 256/512/1024
prefix-cache matrix.

## Environment

- Ferrite commit: `1d30466fee526f938a70278c06ba0defb386d87a`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
- Server: local Ferrite server on `127.0.0.1:18080`
- Server binary SHA256:
  `652393f177907ba1a01e7e72f9dcd131c5701da694117b6f07477bfb9aebfa35`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>

## Model

- Name: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Served model id: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Tokenizer passed to `llama-benchy`: `Qwen/Qwen2.5-0.5B-Instruct`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Server

Command:

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18080 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --experimental-prefix-cache
```

Readiness:

```text
200 {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## llama-benchy Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 128 \
  --tg 32 \
  --depth 128 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:benchy:prefix-smoke-rerun \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-smoke-rerun.json
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-qwen-0-5b-prefix-smoke-rerun.json`

Captured stdout:
`target/proof/llama-benchy-qwen-0-5b-prefix-smoke-rerun.stdout.txt`

## llama-benchy Results

| Phase | Context | Prompt | Response | TG tok/s | TTFR | E2E TTFT | Peak |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Context prefill | 128 | 128 | 32 | 21.219811746997483 | 0.9874590032268316 | 6048.275250010192 | 22.0 |
| Inference | 128 | 128 | 32 | 18.137211810417345 | 1.134749996708706 | 12838.468041998567 | 19.0 |

The command exited `0` and produced the expected two prefix-cache-mode rows:
one context-load row and one inference row.

The `llama-benchy` JSON does not include Ferrite's
`usage.prompt_tokens_details.cached_tokens` field, so this artifact proves
external prefix-mode compatibility but does not by itself prove a cache hit.

## Direct Ferrite Cache-Hit Probe

To verify Ferrite usage metadata against the same live server configuration,
two identical non-streaming chat requests were sent with
`prompt_cache_key=ferrite:direct:prefix-smoke-rerun`.

Raw result:
`documentation/benchmarks/2026-07-02-ferrite-qwen-0-5b-prefix-cache-direct-smoke-rerun.json`

| Request | Prompt tokens | Completion tokens | Cached tokens | Elapsed ms | Finish reason |
| ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 17 | 8 | 0 | 1065.8981669985224 | length |
| 2 | 17 | 8 | 17 | 386.81875000474975 | length |

This proves Ferrite's experimental prefix-cache path reports a full prompt
cache hit for repeated identical chat requests with the same explicit cache
key. It does not prove `llama-benchy` exposes those cached-token fields.

## RSS Sampling

RSS was sampled with `ps -o rss= -p <pid>` once per second while the rerun
server process was alive.

Raw RSS sample:
`target/proof/llama-benchy-qwen-0-5b-prefix-smoke-rerun-rss.tsv`

- Samples: 30
- First sample bytes: 277430272
- Last sample bytes: 476971008
- Peak bytes: 477347840
- Minimum bytes: 168951808

The short run stayed bounded around the same loaded-model RSS range as the
single-model baseline and concurrency steps. The low minimum appears during
startup/loading and is not treated as a steady loaded-model RSS floor.

## Interpretation

`llama-benchy` can exercise its prefix-cache benchmark mode against Ferrite's
OpenAI-compatible server without tool-specific patches. Ferrite's direct
OpenAI-compatible response metadata confirms that the same server build reports
cached prompt tokens on a repeated exact prompt with an explicit
`prompt_cache_key`.

This supports keeping `llama-benchy` in the benchmark harness, but the harness
still needs a companion Ferrite-side cached-token check unless `llama-benchy`
starts exposing usage metadata in its saved results.

## Limits

This does not prove:

- the full 256/512/1024 prefix-cache matrix;
- generated-context long-chat prefix reuse;
- cache correctness for partial-prefix matches;
- cache eviction behavior;
- cache hit reporting inside `llama-benchy` JSON;
- high-concurrency prefix-cache behavior;
- reconnect/error behavior under cache pressure;
- long-running RSS stability.
