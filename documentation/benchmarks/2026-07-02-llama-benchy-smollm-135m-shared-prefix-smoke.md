# Benchmark: llama-benchy SmolLM2 135M Shared-Prefix Smoke

Date: 2026-07-02

## Purpose

Run a bounded external `llama-benchy` prefix-cache benchmark against Ferrite's
OpenAI-compatible server after adding shared-prefix cache reuse.

This benchmark checks external harness compatibility and provides comparative
throughput/latency output. It is paired with a direct Ferrite probe because
`llama-benchy` does not expose Ferrite's
`usage.prompt_tokens_details.cached_tokens` field.

## Environment

- Ferrite commit: `d8d5c6adf86a98373e213c9c279a6ff971669d42`
- Code commit under test: `0a3ecc7070339a1180e20606be9c1898a0f6874f`
- Host: local macOS development machine
- OS: Darwin arm64, `23.5.0`
- CPU: Apple M1 Pro
- Memory: 17179869184 bytes
- Build mode: release
- Server binary SHA256:
  `2528590df4e81a3e0c415ce3f903826055a1a12272ddcf8d960ef48519b244ef`
- External tool: `llama-benchy 0.3.8` via `uvx`
- Source: <https://github.com/eugr/llama-benchy>

## Model

- Name: `SmolLM2-135M-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf`
- Served model id: `smollm2-135m-q4_k_m`
- Tokenizer passed to `llama-benchy`: `HuggingFaceTB/SmolLM2-135M-Instruct`
- Model SHA256:
  `2e8040ceae7815abe0dcb3540b9995eaa1fa0d2ca9e797d0a635ae4433c68c2d`

## Server Command

```sh
target/release/ferrite-server \
  --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf \
  --model-id smollm2-135m-q4_k_m \
  --bind 127.0.0.1:18080 \
  --api-key local-secret \
  --default-max-tokens 32 \
  --hard-max-tokens 128 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness checks passed:

```text
GET /health -> {"status":"ok","ready":true,"model":"smollm2-135m-q4_k_m"}
GET /v1/models -> smollm2-135m-q4_k_m
```

## llama-benchy Command

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18080/v1 \
  --api-key local-secret \
  --model HuggingFaceTB/SmolLM2-135M-Instruct \
  --served-model-name smollm2-135m-q4_k_m \
  --tokenizer HuggingFaceTB/SmolLM2-135M-Instruct \
  --pp 64 \
  --tg 16 \
  --depth 64 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:benchy:smollm135:shared-prefix \
  --format json \
  --save-result documentation/benchmarks/2026-07-02-llama-benchy-smollm-135m-shared-prefix-smoke.json
```

Raw result:
`documentation/benchmarks/2026-07-02-llama-benchy-smollm-135m-shared-prefix-smoke.json`

The command exited `0`.

## llama-benchy Results

| Phase | Context | Prompt | Response | TG tok/s | TTFR | E2E TTFT ms | Peak |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Context prefill | 64 | 64 | 16 | 29.516996154853828 | 0.9257920028176159 | 2241.1231250152923 | 31.48479589851075 |
| Inference | 64 | 64 | 16 | 27.00771816333165 | 1.1289170070085675 | 2275.0255840073805 | 28.80823270755376 |

The saved JSON reports `prefix_caching_enabled=true` and contains the expected
two prefix-cache-mode rows: a context-load row and an inference row.

## Direct Ferrite Shared-Prefix Probe

To verify Ferrite's own cached-token metadata on a divergent prompt pair, two
non-streaming chat requests were sent to the same live server with
`prompt_cache_key=ferrite:direct:smollm135:shared-prefix`.

| Request | Prompt | Prompt tokens | Completion tokens | Cached tokens | Elapsed ms | Finish |
| ---: | --- | ---: | ---: | ---: | ---: | --- |
| 1 | `hello world` | 9 | 4 | 0 | 411.94583399919793 | length |
| 2 | `hello ferrite` | 10 | 4 | 3 | 339.41924999817275 | length |

The second request used a different prompt and still reported
`cached_tokens=3`, proving a shared-prefix cache hit through Ferrite's
OpenAI-compatible response metadata.

## Interpretation

`llama-benchy` can run its prefix-cache mode against the shared-prefix-capable
Ferrite server without tool-specific patches. The direct Ferrite probe confirms
that the same server build reports cached prompt tokens for a divergent
shared-prefix prompt pair.

This supports using `llama-benchy` as an external benchmark harness after
Ferrite's own correctness gate is green.

## Limits

This does not prove:

- generated-context long-chat correctness;
- reconnect/error behavior;
- stop/EOS behavior;
- large-model or x86_64 behavior;
- cache-hit fields inside `llama-benchy` JSON.

Those remain covered by Ferrite's internal long-chat gates and future larger
benchmark runs.
